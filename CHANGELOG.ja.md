# 変更履歴

このプロジェクトの注目すべき変更履歴を記録します。

公開リリース時はセマンティックバージョニングを採用します。

## Unreleased

## 0.3.0 - 2026-07-20

- `kanterm`、`kanterm-mcp`、`kanpty`、`kanptyd`をまとめて導入するchecksum検証付き
  installerを追加。個別version指定とbinary単位のatomicな置換に対応。
- `orient`、`execute`、`verify`、bounded `resume` profileを共有する
  `kanterm-agent-work-packet/v1`と、board / card packetをcopy前に確認するTUI previewを追加。
- 各自動試行の完全なpacket、digest、target、process結果、output、errorを永続化し
  （migration 0021）、retryをbounded resume deltaから構築するように変更。
- agent processの成功と検証済みcard完了を分離。cardのarchiveや次workflow stepの
  triggerには、明示したverification commandの成功を必須化。
- command targetのdelivery、environment、network、workspace、approval、verification、
  writable path policyを検証し、未対応のisolationを暗黙に保証しないように変更。
- Cursor Agent CLIを非対話text modeで起動し、完全なwork packetをprompt引数として渡す
  first-classな`type: cursor` targetを追加。直接変更は明示的なno-prompt policy時だけ有効化。
- `type: interactive`の`adapter: kanpty`を追加。完全なwork packetをprocess引数へ載せず、
  stdin経由のbracketed pasteで稼働中Kanpty session IDまたはaliasへ配送。一時的なbridge
  配送失敗時はhandoffをterminal failureにせずrequeue。
- version付きglobal / project configの自動検出、`kanterm config`の
  path / show / init / edit / validate、headless runnerのtarget / workflow defaultを追加。
- FLOW実行タブを削除し、Kanban、LIST、TIMELINEを`Tab` / `Shift+Tab`および
  `1` / `2` / `3`で切り替える構成に変更。
- LISTとTIMELINEから`b`でボードを切り替え、元の実行viewへ戻れるように変更。
- LISTとTIMELINEから`d`で選択カード、`D`で現在のボードをarchiveでき、確認dialogを
  元の実行view上に維持するように変更。
- 成功したhandoff結果を永続化し、`get_handoff`とsender/status一覧filterを追加。
  MCP clientへ結果取得までのprotocolも注入。

## 0.2.0 - 2026-07-10

- running、human gate、ready、明示的 blocked、dependency waiting、context不足の作業と
  blocker keyをボード単位で確認できる実行ダッシュボードをTUIのfirst viewとして追加。
- Kanban、LIST、dependency stage TIMELINE、導出state FLOWをfirst-class tabとして提供し、
  `Tab` / `Shift+Tab`および`1` / `2` / `3` / `4`で切り替え可能に変更。
- カード詳細を開いた実行タブ上のモーダルとして表示し、閉じたときに元のタブと選択位置を
  復元するように変更。
- dashboard state、input navigation、data projection、renderingの責務を整理し、
  カード詳細を開く際の不要なボード再読込を削除。
- 端末のデフォルト背景を使う `glass` テーマを既定にし、TUI header、列間余白、
  選択 marker、responsive key hint を刷新。

## 0.1.0 - 2026-07-09

- **破壊的変更 (MCP):** `create_card` と `create_cards` で `board` を必須化。
  既存のプロジェクトボード slug を渡すとそのボードへ、未知の名前を渡すと
  workflow テンプレートのボードを新規作成してカードを追加します。`board` を
  省略した場合は、従来の Backlog への暗黙フォールバックではなくエラーになります。
- MCP ツール `create_card_in_backlog` を追加。Backlog インボックスは明示的な
  オプトイン専用となり、`create_card`/`create_cards` からは到達できません。
- `create_card`/`create_cards` のレスポンスに、作成先ボードの slug と、ボードが
  既存か新規作成かを含めるようにしました。
- ローカル TUI kanban ボードを SQLite で提供
- 自動化クライアント向け MCP サーバを追加
- 記憶ログの追加（参照履歴と月次クリーンアップ）
- workflow handoff field とアドバイザリ的な貸し出しロックを追加
- テーマ、ボード順序、計画レーンを追加
- Rust package 名を project 名に合わせて `kanterm-core`、`kanterm`、
  `kanterm-mcp` に統一
- MIT ライセンスと公開準備用ドキュメントを追加
