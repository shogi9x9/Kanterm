# kanban-tui

English version: [README.md](README.md)

terminal UI と MCP server を持つ、ローカル専用のタスク実行ストアです。
人間は TUI で視覚確認・監査・介入し、AI エージェントは MCP で同じカードを実行状態として扱います。

- **すぐ起動・すぐ終了**: 単一の Rust ネイティブバイナリで、通常の terminal から
  直接起動できます。tmux popup binding は任意の launcher として用意しています。
- **単一情報源**: TUI も MCP も同じ `kanban-core` と同じ SQLite データベースを使うため、
  見えている内容と更新対象が一致します。
- **エージェント向け**: `kanban-mcp` が TUI と同じカードを読み書きします。
- **実行状態の永続化**: handoff field、実行メタデータ、claim、検証結果、
  board-level 実行規約、memory を残し、長い作業の再開と監査をしやすくします。

## 関連ドキュメント

- 設計: [DESIGN.md](DESIGN.md) / [DESIGN.ja.md](DESIGN.ja.md)
- 貢献: [CONTRIBUTING.md](CONTRIBUTING.md) / [CONTRIBUTING.ja.md](CONTRIBUTING.ja.md)
- リリース: [RELEASE.md](RELEASE.md) / [RELEASE.ja.md](RELEASE.ja.md)
- セキュリティ: [SECURITY.md](SECURITY.md) / [SECURITY.ja.md](SECURITY.ja.md)
- 変更履歴: [CHANGELOG.md](CHANGELOG.md) / [CHANGELOG.ja.md](CHANGELOG.ja.md)

## ビルド

```sh
cargo build --release
```

成果物: `target/release/kanban-tui`, `target/release/kanban-mcp`

## 使用方法

### TUI

```sh
./target/release/kanban-tui
```

### MCP（for agents）

プロジェクト内にある `.mcp.json` で `kanban` サーバが自動登録されます。

代表的な操作:

- `status`: version / schema / DB path / working directory / 既定ボードを表示
- `get_board`: ボード全体表示
- `list_cards`: クエリ/状態/列で一覧（`query` は FTS5-backed card search index を使用）
- `get_card`: カード詳細
- `create_card`: カード作成
- `create_cards`: spec / plan から複数カードを順序付きで作成
- `dependency_graph`: 依存 edge、実行 stage、依存 block を表示
- `register_agent`: `codex` などの希望名を登録し、`codex#abc123` のような割当 identity と claim token を取得
- `update_card`: カード更新（列/ボード移動、handoff field、実行メタデータ、claim など）
- `manage_columns`: 列の追加/変更/削除/順序入れ替え
- `manage_boards`: ボードの作成（`name`、任意の `template`、任意の `agent_context`。既定 template は `workflow`）/アーカイブ/復元/順序変更/board-level context 設定
- `record_memory`: 意思決定ノートの登録
- `recall_memories`: ノート検索

既定の **Backlog** ボード（slug: `backlog`）は予約済みの inbox / planning 用ボードです。
このボードには **Backlog** 列 1 本だけが存在し、列の追加/変更/削除/順序入れ替えはできません。
また、`Backlog` という名前のボードを追加で作成することもできません。

プロジェクト用に新規作成するボードは、開発者向けの `workflow`
（**Todo / In progress / Testing / Waiting for release**）が既定です。
必要に応じて `planning`（**Backlog / Today / This week / This month**）や
`simple`（**Todo / Doing / Done**）も選択できます。
作成後の列は `manage_columns` や TUI の列管理で変更できます。
TUI ではカード選択中に `M` を押すと、移動先ボードと列を選んでカードを別ボードへ移動できます。
Backlog ボード上ではこの操作を `send-project` と表示し、inbox からプロジェクトへ送る操作として扱います。
ボード横断移動は旧 key / 新 key / 移動元ボード / 移動先ボードを activity に記録するため、
`get_card` で key の付け替わりを追跡できます。
TUI のボード画面では `i` で現在のボードの `agent_context` を編集できます（空入力でクリア）。
TUI の詳細画面で `x` を押すか MCP の `complete_note` を使うと、カードはアーカイブされ、
`agent_state=done` になり、進行中の handoff field と claim がクリアされます。
TUI のボード画面で `u` を押すと、直近の取り消し可能なカード更新（誤アーカイブ、完了、カード移動など）を戻せます。
完全削除は意図的に undo 対象外です。
agent がカードを claim する場合は先に `register_agent` を呼び、返された `assigned_identity`
を `update_card.claim` に、`claim_token` を claim / renew / release 操作に渡します。
ボードには `agent_context` も保存できます。これはカード単位の `next_action` とは別に、
検証コマンド、完了条件、repo 固有の規約、release gate などを board/project 単位で置くための
agent 向け実行規約です。`manage_boards(action="set_context", board="slug",
agent_context="...")` で設定し、作成時にも `agent_context` を渡せます。
設定済みの場合、`get_board` と `get_card` に表示されます。

agent 実行の基本フローは、spec / plan から `create_cards` で永続カードを作成し、
`acceptance_criteria`、`next_action`、実行メタデータを付け、`list_cards(queue="executable")`
で次に実行可能なカードを選び、`get_card` で詳細確認してから claim する流れです。
実行後は `last_verification` を更新し、`complete_note` で完了、必要に応じて
`record_memory` で判断理由を残します。

`priority` は人間/事業上の優先度です。agent 向けの実行判断は別フィールドで表します。
`agent_weight` は agent 実行コスト/適性（1..5）、`agent_effort` は推論・実行負荷、
`suggested_model` はモデル候補、`expected_tokens` は想定 token 量、
`human_intervention` は `none` / `review` / `decision` / `execution` の人間介入状態です。
`list_cards` は `queue=executable` / `review` / `blocked` / `claimed` /
`missing_context` / `dependency_blocked` / `human` で実行候補を分けられます。
`ranked=true` を付けると、次に着手しやすい順に並べ替え、簡潔な ranking 理由を表示します。
依存グラフは first-class data です。`create_cards` の各カードに `alias` と
`depends_on` を付けると、`dependency_graph` が明示的な edge と実行stageを表示します。
`active_only=true` で完了済みの履歴 edge を隠し、`focus="KEY"` で特定カードと
その直接の上下流だけを確認できます。
たとえば `A -> B/C/D 並列 -> E` は次のように表せます。

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

定期メンテナンスも同じ永続カードの流れで扱います。単に「docs cleanup」のような曖昧な
メモを残すのではなく、claim と検証が独立してできる具体カードに分けます。ローカル運用では
次のような `create_cards` パターンを使います。

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
      "next_action": "Compare crates/kanban-mcp/src/instructions.rs, tool descriptions, and get_board board_agent_context; create or patch exact drift.",
      "agent_weight": 2,
      "agent_effort": "medium",
      "expected_tokens": 2500
    }
  ]
}
```

ボード横断移動の手順は、既存ドキュメントを参照:

- 日本語: [docs/mcp-card-migration.ja.md](docs/mcp-card-migration.ja.md)
- 英語: [docs/mcp-card-migration.en.md](docs/mcp-card-migration.en.md)

### バックアップ / 復元

復元用途では SQLite DB レベルのバックアップを使います。`--export` はレビューしやすい
スナップショット出力であり、完全な import 用ではありません。

```sh
./target/release/kanban-tui --backup-db ./kanban-backup.db
./target/release/kanban-tui --restore-db ./kanban-backup.db --force
```

バックアップは SQLite の `VACUUM INTO` で作成するため、WAL に残っている内容も一貫した形で含まれます。
復元時は入力DBの schema version を検証し、既存DBの置き換えには `--force` を必須にします。
より新しい kanban で作られたバックアップは、古いバイナリでは復元を拒否します。

## リリース向けチェック

OSS 公開を想定する場合は `RELEASE.md` を参照してください。

## 開発方針

本プロジェクトは個人開発ベースで保守しています。
**Pull Request は受け付けません。**
バグ報告、機能要望、質問は Issue で受け付けます。
