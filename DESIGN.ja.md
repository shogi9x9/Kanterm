# デザイン

English version: [DESIGN.md](DESIGN.md)

terminal UI と MCP server を持つローカル専用のタスク実行ストアを、AI エージェントと共有する
設計を、採択済みの方針に沿って整理したものです。TUI は人間の監査/介入パネル、
MCP は agent の実行面として扱います。

## 目標と制約

- **ローカル専用・単一ユーザー**。クラウド、サーバ、認証は前提にしない。
- **Terminal UX**。通常の terminal で直接起動でき、tmux popup binding などの任意
  launcher からも開ける。何度も起動・終了するため、起動体感を軽く保つ。
- **エージェントとデータ整合**。TUI で見えるカードと MCP で編集されるカードは
  同一データを共有し、同時編集を許容する。
- **実行状態の永続化**。handoff、実行メタデータ、claim、検証結果、board-level
  実行規約、memory を残し、再起動や context compaction 後も再開できるようにする。

## 技術選定

| 観点 | 選定 | 理由 |
| --- | --- | --- |
| 言語 / TUI | Rust + ratatui | 単一ネイティブバイナリで terminal 起動が速い |
| 永続化 | rusqlite を使った SQLite (WAL) | 読み取り/書き込み同時運用、トランザクション可 |
| MCP | rmcp (stdio) | 公式 Rust SDK を使い、言語統一で実装負荷を抑制 |

Node/Python は起動遅延が大きく、頻繁に開くローカル tool には合わないため採用しません。

## クレート分割

- `kanterm-core`: スキーマ、マイグレーション、ドメインロジックと書き込みロジックを
  一元管理する。
- `kanterm`: ratatui ベースのフロントエンド。同期実装。
- `kanterm-mcp`: 非同期 rmcp サーバ。`Store` を Mutex で握って `kanterm-core` を呼び出す。

## ボードモデル

- 既定の **Backlog** ボードは `backlog` slug を持つ予約済みの inbox / planning ボード。
- Backlog ボードは **Backlog** 列 1 本だけを持ち、列の追加/変更/削除/順序入れ替えを拒否する。
- `Backlog` という名前の追加ボードは作成できない。
- プロジェクト用ボードは列テンプレートから作成する。開発者向けの既定は
  `workflow`（**Todo / In progress / Testing / Waiting for release**）。明示的に
  `planning`（**Backlog / Today / This week / This month**）や
  `simple`（**Todo / Doing / Done**）も選べる。作成後の列は列管理の対象になる。
- Backlog ボードからプロジェクトボードへ移すときは、MCP では `update_card.move_to_board`
  を使う。
- TUI ではカード選択中に `M` で移動先ボード picker と列 picker を開き、同じ core
  更新経路でボード横断移動する。
- 保護された Backlog ボード上では同じ操作を `send-project` と表示し、inbox から
  プロジェクトボードへ送るトリアージ操作として見せる。
- 詳細画面の `x` は任意の完了メモ付きでカードを完了し、アーカイブ、`agent_state=done`、
  進行中 handoff field のクリア、claim 解除をまとめて行う。

## 同時実行モデル

短命な TUI と長寿命の MCP サーバが同時書き込みし得るため、
以下を `kanterm-core` で担保しています。

- `journal_mode = WAL`
- `busy_timeout = 5000ms`
- 全書き込みを `BEGIN IMMEDIATE` で開始
- `PRAGMA foreign_keys = ON`

TUI 側は 800ms のアイドルポーリングで `PRAGMA data_version` を監視し、
外部更新があったときのみ再ロードします（上限約 800ms）。

## スキーマ/マイグレーション

- `PRAGMA user_version` ベースの順次 SQL マイグレーションを採用。
- 開封時に未適用分を `IMMEDIATE` トランザクションで実行。
- DB バージョンが実行バイナリより新しい場合は起動を失敗させます。

主要テーブル: `boards`, `columns`, `cards`, `labels`, `card_labels`,
`activity_logs`, `ui_state`, `agent_registrations`。

重要設計: `cards.position REAL` の分数インデックス、`cards.updated_at` の
楽観的排他準拠。

`activity_logs` は `actor` (`tui` / `agent`) を持たせているため、
人間とエージェントの更新履歴を混在で追跡します。
ボード横断移動では `move_board` activity に旧 key / 新 key / 移動元ボード /
移動先ボードを構造化して記録し、移動先で key が再発行されても追跡できるようにします。
`card_search` は core の書き込み経路で同期される FTS5 virtual table です。
`list_cards(query=...)` は title / body / labels / agent workflow fields をこの index で検索し、
短い語や記号を含む query では従来の literal substring fallback も併用します。

## MCP 仕様

初期は最小限のツール構成を採用。主要ポイントは以下。

- コア操作は `get_board`, `list_cards`, `get_card`, `create_card`, `create_cards`, `update_card`
- agent は `register_agent` で `codex#abc123` のような割当 identity と
  claim token を取得し、claim / renew / release 時に token を渡す
- `move` は `update_card` の `column` で実現
- `search` は `list_cards(query=...)`
- 次に実行できる作業の選択は `list_cards(queue=...)`
- board/project 単位の検証コマンド、完了方針、repo 固有規約、release gate は
  `manage_boards` の `agent_context` に保存し、カード単位の `next_action` より上位の
  実行規約として扱う
- キー参照 (`KB-12`) のみを公開し、内部 ID は外部露出しない
- 戻り値は JSON ではなくテキスト/Markdown

## agent 実行モデル

- `priority` は人間/事業上の優先度を表す。
- `agent_weight` は agent 実行コスト/適性を `1..5` で表す。
- `agent_effort` は推論・実行負荷、`suggested_model` はモデル候補、
  `expected_tokens` は想定 token 量を表す。
- `human_intervention` は `none` / `review` / `decision` / `execution` で、
  自律実行できるか、人間の確認や判断が必要かを分ける。
- spec / plan は `create_cards` で順序付きの永続カードにし、
  `alias` と `depends_on` でDAGを表現し、`acceptance_criteria`、
  `next_action`、実行メタデータを付ける。
- agent は `list_cards(queue="executable")` で候補を選び、`get_card` で詳細確認、
  `register_agent` と claim 後に実行する。
- 実行後は `last_verification` と `complete_note`、必要なら `record_memory` で
  検証結果と判断理由を残す。

依存グラフは first-class data です。`dependency_graph` でedgeとstageを確認でき、
`A -> B/C/D 並列 -> E` のような fan-out / fan-in を表現できます。

## 信頼性と undo

- undo 可能なカード更新は、更新前のカード全体を activity payload に保存します。
  TUI の `u` はアクティブボード上の直近の対象更新を復元します。完全削除は意図的に
  undo 対象外です。
- TUI の編集モードは開始時の `updated_at` を保持し、保存時に
  `expected_updated_at` として渡します。外部更新と競合した場合は、暗黙に上書きせず
  更新を拒否します。
- 本文エディタは安全な文字列操作のため cursor を文字 index で保持し、描画時だけ
  cursor より前の文字列を terminal display width へ変換します。CJK と半角文字が
  混在しても caret 位置がずれません。

## agent handoff orchestration

- 永続 handoff（migration 0019）はカードの workflow field とは別の leased inbox
  queue です。登録済み agent の exact identity または agent family 宛てに送り、
  `send_handoff` / `list_handoffs` / `claim_handoff` / `complete_handoff` で管理します。
- `watch-handoffs` は inbox item を claim し、command target または薄い bridge script
  へ配送します。target YAML で routing を再利用できます。tmux / zellij の
  interactive target shape は parse 済みですが、adapter 実装は保留中です。
- 小さな workflow YAML は named step と `on_complete.send_handoff` を扱います。
  カード完了から同じ runner を起動でき、`run-agent-task` は incoming handoff の claim、
  command target 実行、カード完了、次 step の任意 trigger を一続きにします。
- Claude Code hook installer は Kanterm 所有の `SessionStart` / `SessionEnd` / `Stop`
  entry だけを管理し、他の hook を保持します。

## 実行ダッシュボード

- TUI は全ボードのアクティブカードを横断する primary view から起動し、ボードから
  `W` で切り替えます。running、human gate、ready、明示的 blocked、dependency waiting、
  context不足に分けます。title と why/next を別列にし、通常の terminal 幅でも
  dependency key、blocker reason、next action を見えるようにします。
- dashboard の分類は `kanterm-core::classify_work` を呼び、local next-work navigation と
  MCP queue filtering と同じ優先順位を使います。TUI 側で判定を重複させません。
- `j` / `k` で実行リストを移動し、`Enter` で対象 board へ切り替えてカード詳細を
  開きます。外部 MCP 更新は既存の `data_version` refresh でdashboardにも反映します。
- `Tab`でLIST / TIMELINE / FLOWを巡回し、`1` / `2` / `3`で直接選択します。
  TIMELINEは`dependency_stage_plan`をガント風stage軸へ写し、calendar durationを
  仮定せず並列カードを同じ列に置きます。FLOWは`classify_work`のlive countを
  state-machine風mapへ置き、選択stateのカードを表示します。どちらも実行policyを
  所有せず、`kanterm-core`のread projectionとして扱います。

## 既知の follow-up

- 楽観的排他エラー時に、外部版の再読込かローカル編集の再開を選べる TUI recovery
  prompt を追加する。
- runtime 固有状態を `kanterm-core` に入れず、予約済み tmux / zellij interactive
  target adapter を実装する。
- retry / timeout / cancel を増やす前に workflow run と step state を永続化する。
  現在の runner は汎用 CI engine ではなく、小さな handoff trigger に保つ。
- `rmcp`（`=1.7.0` 固定）は version 更新ごとに再検証する。

## リリース時の注意

`DESIGN` と実装の更新を合わせ、変更内容を常に設計文書へ反映します。
