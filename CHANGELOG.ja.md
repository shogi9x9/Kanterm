# 変更履歴

このプロジェクトの注目すべき変更履歴を記録します。

公開リリース時はセマンティックバージョニングを採用します。

## Unreleased

- running、human gate、ready、明示的 blocked、dependency waiting、context不足の作業を
  横断表示し、blocker key の確認とカード詳細への直接移動ができる実行ダッシュボード
  （`W`）をTUIのfirst viewとして追加。`Tab`および`1` / `2` / `3`で切り替えるLIST、
  dependency stage TIMELINE、導出state FLOWを提供。
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
