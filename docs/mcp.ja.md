# MCP リファレンス（for agents）

English version: [mcp.md](mcp.md)

プロジェクト内の [`.mcp.json`](../.mcp.json) により、このディレクトリで実行すると
Claude Code が `kanterm` サーバを自動登録します。

## ツール

カードは key（例: `KB-12`）で指定します。

| ツール        | 用途                                                |
| ------------- | --------------------------------------------------- |
| `status`      | 読み取り専用のサーバ状態: version / schema / DB path / working directory / 既定ボード |
| `get_board`   | 概要表示: 各列とそのカード（最初に呼ぶ）            |
| `list_cards`  | カードを 1 行ずつ。列/状態/クエリで絞り込み（`query` は FTS5-backed card search index を使用） |
| `get_card`    | 1 枚のカードを本文込みで全表示                      |
| `create_card` | カード作成（`title`、任意の `body`・`column`）       |
| `create_cards` | spec / plan から順序付きで複数カードを作成。実行メタデータも任意で付与 |
| `dependency_graph` | 依存 edge・実行 stage・blocker を表示          |
| `register_agent` | エージェント表示名を要求し、`codex#abc123` のような割当 identity と claim token を取得 |
| `send_handoff` | exact identity または agent family name 宛てに永続 inbox message を送信 |
| `list_handoffs` | open handoff を列挙。任意で 1 つの agent inbox に絞り込み |
| `claim_handoff` | agent identity と claim token で handoff を claim し、復旧可能な lease を設定 |
| `complete_handoff` | claim 済み handoff を `completed` または `failed` にする |
| `update_card` | 任意フィールドを更新。`column` で列移動、`move_to_board`（slug）で別ボードへ移動、`add_labels` / `remove_labels` でタグ付け、`due`（`YYYY-MM-DD`、`""` でクリア）で期日、`next_action` / `blocked_reason` / `acceptance_criteria` で handoff 状態、実行メタデータで適性、`claim` / `claim_token` / `release_claim` / `lease_minutes` でエージェント所有権を調整 |
| `manage_columns` | プロジェクトボードの列を追加 / 変更 / 削除（移送先 `to` 付き）/ 並べ替え。Backlog ボードは列変更を拒否 |
| `manage_boards` | ボードを作成（`name`、任意の `template`（既定 `workflow`）、任意の `agent_context`）/ アーカイブ / 復元 / 並べ替え（slug 指定）。`set_context` / `clear_context` でボード単位の実行規約を保存。`Backlog` は予約済み、`delete` はアーカイブ済みプロジェクトボードのみ |
| `record_memory` | セッションを跨いで残る意思決定/学習/文脈ノートを登録。任意で `card` 連携（例 `KB-12`） |
| `recall_memories` | memory を `query`/`card`/`kind` で検索（新しい順）。`key`（例 `M-3`）で 1 件を全表示 |

カード/列系ツールはいずれも任意の `board` slug を取り（既定は Backlog ボード、
slug は `backlog`）、`get_board` は末尾に全ボードを列挙するため、エージェントは
ボードを発見して対象にできます。スケジュール可能になったカードは `move_to_board`
で Backlog からプロジェクトボードへ移します。TUI ではこの Backlog 操作を
`send-project` と表示します。
ボード横断移動は旧 key / 新 key / 移動元ボード / 移動先ボードを activity に記録
するため、`get_card` で key の付け替わりを追跡できます。

ラベルは名前で初めて参照したときに自動作成され、`get_board` / `list_cards` /
`get_card` にインラインで表示されます。期日は `due:YYYY-MM-DD` として描画され、
過去日には `!` 接頭辞（`get_card` では `(overdue)`）が付きます。

エージェントは claim の前に `register_agent` を呼び、返された `assigned_identity`
を `update_card.claim` に、`claim_token` を claim / renew / release に渡します。
有効な lease は `[claimed:codex#abc123]`、期限切れは `[claim-expired:name]` と
描画され、別の登録済みエージェントが引き継げます。
`complete_note` はカードをアーカイブし、メモが空でなければ追記し、
`agent_state=done` にして、進行中の handoff field と claim をクリアします。

agent 間 handoff はカード内 field とは別の永続 queue です。`send_handoff` は同じ
SQLite DB に inbox item を保存し、exact identity（`claude#abc123`）または agent
family（`claude`）に宛てられます。受信側 agent、hook、watcher は
`list_handoffs(for_agent="claude#abc123")` で確認し、`claim_handoff` で作業を取得し、
`complete_handoff` で閉じます。lease があるため、受信側が動く前に終了しても別の
watcher が期限切れ handoff を回収できます。runtime 固有の hook / bridge はこの
queue の薄い配送層に寄せ、状態本体は Kanterm に永続化します。

`kanterm-mcp` には配送層向けの headless watcher も含まれます。

```sh
kanterm-mcp watch-handoffs \
  --for-agent claude#abc123 \
  --claim-token "$CLAIM_TOKEN"
```

watcher は Kanterm DB を polling し、該当 handoff を claim して、claim 済み
handoff を 1 件 1 行の JSON として stdout に出力します。配送に成功すると handoff
は `completed` になります。hook / test 用には `--once` で 1 回だけ scan できます。
`--interval-ms` で polling interval を変更できます。
agmsg の monitor watcher と同じ考え方で、各 watcher は `KANTERM_RUN_DIR` または
既定の temp run directory に pidfile と ready sentinel を書きます。ready file は
`watch.<agent>.ready` で、watcher が slot を claim し、初回 scan を終えたあとに
作成されます。同じ agent 向けの 2 つ目の watcher は、先行 process が生きている間
は失敗します。置き換える場合は `--replace-existing` を渡します。supervisor が
安定した path を待つ場合は `--run-dir DIR` を指定します。`Stop` hook fallback の
ように monitor watcher が動いていれば静かに終了したい場合は `--skip-if-running`
を使います。

別 runtime へ渡す場合は bridge command を指定します。

```sh
kanterm-mcp watch-handoffs \
  --for-agent claude#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --run-dir /tmp/kanterm-run \
  --bridge-command ./scripts/kanterm-bridge-file-inbox.sh \
  --bridge-arg --repo \
  --bridge-arg /path/to/downstream-repo
```

bridge は handoff body を stdin で受け取り、`KANTERM_HANDOFF_ID`、
`KANTERM_HANDOFF_FROM_AGENT`、`KANTERM_HANDOFF_TO_AGENT`、
`KANTERM_HANDOFF_SUBJECT`、`KANTERM_HANDOFF_CARD_KEY`、
`KANTERM_HANDOFF_LEASE_EXPIRES_AT` などの環境変数で metadata を受け取ります。
exit code 0 なら `completed`、非 0 なら bridge error 付きで `failed` になります。
Kanterm には汎用 bridge として、target repo の Markdown inbox に保存する
`scripts/kanterm-bridge-file-inbox.sh` と、target repo で任意 command を起動し
handoff prompt を stdin で渡す `scripts/kanterm-bridge-agent-command.sh` が含まれます。

配送先を繰り返し使う場合は、小さな YAML config に target を定義し、
bridge 引数を毎回書く代わりに `--targets PATH --target NAME` を渡せます。

```yaml
targets:
  - name: bff-command
    type: command
    agent: bff-agent
    repo: /path/to/downstream-repo
    command: claude
    args: -p

  - name: bff-session
    type: interactive
    agent: bff-agent
    adapter: tmux
    session: bff
    pane: 0
```

```sh
kanterm-mcp watch-handoffs \
  --for-agent bff-agent#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --targets ./kanterm.targets.yaml \
  --target bff-command
```

`type: command` は実装済みです。Kanterm は target の `repo` で `command` と
`args` を起動し、整形済み handoff prompt を stdin に渡します。`type:
interactive` は tmux / zellij session adapter 向けの予約済み target 形状です。
config としては検証されますが、adapter 実装が入るまでは watcher delivery は
明示的な未実装エラーを返します。

cross-repo orchestration では、`kanterm-mcp run-workflow` で workflow step の完了を
永続 handoff に変換できます。これは意図的に小さな YAML subset です。named step と
`on_complete.send_handoff` action だけを扱い、配送は同じ watcher / bridge 層に任せます。

```yaml
name: ms-to-bff
initial_step: implement_ms
steps:
  - name: implement_ms
    agent: ms-agent
    on_complete:
      send_handoff:
        target: bff-command
        subject: Continue {{card}} for {{workflow}}
        body: Continue {{step}} from {{from_agent}} into {{repo}}
```

```sh
kanterm-mcp run-workflow \
  --workflow ./kanterm.workflow.yaml \
  --targets ./kanterm.targets.yaml \
  --from-agent ms-agent \
  --board ms \
  --card MS-1
```

runner は `{{workflow}}`、`{{step}}`、`{{step_agent}}`、`{{from_agent}}`、
`{{target}}`、`{{to_agent}}`、`{{repo}}`、`{{board}}`、`{{card}}` を展開します。
`send_handoff.to_agent` は従来どおり直書きできます。`send_handoff.target` を使う場合、
宛先は target の `agent` から解決され、未設定なら target 名に fallback します。
`--board` と `--card` を渡すと、その Kanterm card に紐づく handoff として保存され、
通常の card validation も適用されます。

同じ workflow は、カード完了時の `update_card` から直接 trigger できます。workflow
trigger field は `complete_note` と一緒に渡した場合だけ受け付けるため、通常の編集で
誤って downstream work を enqueue することはありません。

```json
{
  "board": "ms",
  "key": "MS-1",
  "complete_note": "implemented and verified",
  "workflow": "./kanterm.workflow.yaml",
  "workflow_targets": "./kanterm.targets.yaml",
  "workflow_from_agent": "ms-agent"
}
```

成功すると、`update_card` response に `workflow_triggered:` と `run-workflow` と同じ
summary が追記されます。trigger は完了した card の board / key を使うため、workflow
template 側では `{{board}}` と `{{card}}` をそのまま利用できます。

受信側が外側の script に `update_card` を手動実行してもらうのではなく、自分で chain
を進める場合は `kanterm-mcp run-agent-task` を使います。これは incoming handoff を
1 件 claim し、設定済み command target を実行し、その出力を completion note として
指定 Kanterm card を完了し、必要なら次の workflow step を trigger します。

```sh
kanterm-mcp run-agent-task \
  --for-agent b#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --targets ./kanterm.targets.yaml \
  --target b-command \
  --board ms \
  --card MS-2 \
  --workflow ./kanterm.workflow.yaml \
  --workflow-targets ./kanterm.targets.yaml \
  --workflow-step b-to-c \
  --from-agent b
```

この runner は headless command agent 用の loop です。incoming handoff は watcher と
同じ claim/token/lease で扱い、受信 agent の command target を実行し、Kanterm card
完了後に workflow trigger で次 agent へ handoff します。

Claude Code 向けには、Kanterm が project-local hooks を
`.claude/settings.local.json` に install できます。

```sh
kanterm-mcp hooks install \
  --runtime claude-code \
  --mode both \
  --for-agent claude#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --run-dir /tmp/kanterm-run \
  --bridge-command ./scripts/kanterm-bridge-file-inbox.sh \
  --bridge-arg --repo \
  --bridge-arg /path/to/downstream-repo
```

installer は idempotent です。既存の Kanterm-owned `SessionStart` / `SessionEnd` /
`Stop` entry を削除してから、選択 mode の entry を追加します。Kanterm 管理外の
hook は保持します。mode は agmsg の delivery 分割と同じで、`monitor` は
`SessionStart` / `SessionEnd` の background watcher hooks、`turn` は turn 間に
1 回だけ scan する `Stop` fallback、`both` は両方、`off` は Kanterm-owned hooks
の削除です。`both` の `Stop` fallback は `--skip-if-running` を使うため、live
monitor watcher と競合しません。現在の mode は `kanterm-mcp hooks status`、削除は
`kanterm-mcp hooks uninstall` で確認・実行できます。

ボードには `agent_context` も保存できます。これはカード単位の `next_action` とは
別に、検証コマンド・完了条件・repo 固有の規約・release gate などをボード/プロジェ
クト単位で置くための実行規約です。`manage_boards(action="set_context",
board="slug", agent_context="...")` で設定し、作成時にも渡せます。設定済みの場合、
`get_board` と `get_card` に表示され、エージェントは個々のカードの `next_action`
を実行する前にボードのルールを把握できます。

## エージェント実行フロー

代表的なフローは次のとおりです。

1. spec / plan を `create_cards` で永続カードにする。
2. DAG の場合は `alias` と `depends_on` を付ける。
3. 各カードに実行文脈（`acceptance_criteria`、`next_action`、任意の実行メタ
   データ）を載せる。
4. `dependency_graph` で stage を確認し、`list_cards(queue="executable")` で
   作業を取得する。
5. `get_card` で詳細を確認し、`register_agent` してカードを claim する。
6. 実行し、`last_verification` を更新し、`complete_note` で完了する。
7. 別 agent / 別 project に続きの作業を渡す場合は `send_handoff` を使う。
8. 自明でない判断は `record_memory` で残す。

`priority` は人間/事業上の優先度マーカー（`[L]`、`[M]`、`[H]`）のままです。
実行メタデータはこれとは別です。

- `agent_weight`: エージェントの適性/コスト（`1..5` の小さな尺度）。
- `agent_effort`: 要求する推論/実行負荷（`low`・`medium`・`high-reasoning` など）。
- `suggested_model`: タスク向けのモデル/プロファイル候補。
- `expected_tokens`: 想定 token 量。
- `human_intervention`: `none` / `review` / `decision` / `execution`。

queue フィルタは自律作業と人間ゲート作業を分けます: `queue=executable`、
`queue=review`、`queue=blocked`、`queue=claimed`、`queue=missing_context`、
`queue=dependency_blocked`、`queue=human`。`ranked=true` を付けると、次に着手し
やすい順に並べ替え、簡潔な ranking 理由を表示します。
依存グラフは first-class data です。`dependency_graph` は明示的な edge と実行
stage（例 `A -> B/C/D 並列 -> E`）を描画します。`active_only=true` で完了済みの
履歴 edge を隠し、`focus="KEY"` で特定カードとその直接の上下流だけを確認できます。

## import 例

最小の fan-out / fan-in import:

```json
{
  "board": "plan",
  "cards": [
    { "alias": "A", "title": "A", "acceptance_criteria": "A done", "next_action": "do A" },
    { "alias": "B", "title": "B", "depends_on": ["A"], "acceptance_criteria": "B done", "next_action": "do B" },
    { "alias": "C", "title": "C", "depends_on": ["A"], "acceptance_criteria": "C done", "next_action": "do C" },
    { "alias": "D", "title": "D", "depends_on": ["A"], "acceptance_criteria": "D done", "next_action": "do D" },
    { "alias": "E", "title": "E", "depends_on": ["B", "C", "D"], "acceptance_criteria": "E done", "next_action": "do E" }
  ]
}
```

定期メンテナンスも同じ永続カードの流れで扱います。「docs cleanup」のような曖昧な
メモを 1 つ残すのではなく、claim と検証が独立してできる具体カードに分けます。
ローカル運用では次のようなパターンが有用です。

```json
{
  "board": "kanban-improvements",
  "cards": [
    {
      "alias": "refactor-scan",
      "title": "Maintenance: scan refactor pressure",
      "acceptance_criteria": "Oversized modules, duplicated policy, or stale helpers are either recorded as concrete follow-up cards or explicitly judged acceptable.",
      "next_action": "Compare current diff and module boundaries, then create focused follow-up cards for real refactor pressure.",
      "agent_weight": 2,
      "agent_effort": "medium",
      "expected_tokens": 2500
    },
    {
      "alias": "readme-parity",
      "title": "Maintenance: README English/Japanese parity",
      "acceptance_criteria": "README.md and README.ja.md describe the same public behavior, tool names, and release caveats.",
      "next_action": "Diff README.md against README.ja.md and patch only behavior drift.",
      "agent_weight": 1,
      "agent_effort": "low",
      "expected_tokens": 1500
    },
    {
      "alias": "design-parity",
      "title": "Maintenance: DESIGN English/Japanese parity",
      "acceptance_criteria": "DESIGN.md and DESIGN.ja.md reflect the same architecture decisions and shipped behavior.",
      "next_action": "Compare design docs against current core/MCP/TUI behavior and patch drift.",
      "agent_weight": 1,
      "agent_effort": "low",
      "expected_tokens": 1500
    },
    {
      "alias": "agent-surface-drift",
      "title": "Maintenance: MCP instructions and board context drift",
      "acceptance_criteria": "MCP server instructions, tool descriptions, and board agent_context match the current agent execution flow.",
      "next_action": "Compare crates/kanterm-mcp/src/instructions.rs, tool descriptions, and get_board board_agent_context; create or patch exact drift.",
      "agent_weight": 2,
      "agent_effort": "medium",
      "expected_tokens": 2500
    }
  ]
}
```

メタデータの具体例:

- 広範なリファクタ: 高い `agent_weight`・高い `agent_effort`・明示的な
  `acceptance_criteria`、必要なら `human_intervention=review`。
- UI 判断: `human_intervention=decision` または `review`。
- docs 整理: 低 weight・低 effort・小さな token 予算。
- 曖昧なプロダクト判断: `human_intervention=decision`。
- 高 token のリサーチ: 高い `expected_tokens`、より強いモデルを suggest。
- 機械的な編集: 低 weight・低 effort・`human_intervention=none`。

## stdio での手動確認

```sh
KANBAN_DB=/tmp/k.db ./target/release/kanterm-mcp
# その後 JSON-RPC を送る: initialize → notifications/initialized → tools/list
```

## ボード移行

- 日本語: [mcp-card-migration.ja.md](mcp-card-migration.ja.md)
- English: [mcp-card-migration.en.md](mcp-card-migration.en.md)
