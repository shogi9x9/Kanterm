# Kanterm

> ターミナルで使える kanban ボード。必要に応じて MCP interface から自動化できます。

[![CI](https://github.com/shogi9x9/Kanterm/actions/workflows/ci.yml/badge.svg)](https://github.com/shogi9x9/Kanterm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

English version: [README.md](README.md)

![Kanterm demo](docs/assets/demo.gif)

Kanterm は、単一の SQLite データベースの上に 2 つのフロントエンドを持つ、
シングルユーザー向けのタスクボードです。日常の操作には **terminal UI** を使い、
必要に応じて **MCP サーバ** からスクリプトや外部ツールで同じカードを読み書き
できます。両者は同時に動作し、互いの書き込みをライブで反映します。

## 特長

- **1 つのボード、2 つの面**: TUI も MCP サーバも同じ `kanterm-core` と同じ
  SQLite（WAL）データベースを通すため、見ている内容と更新対象が常に一致します。
- **単一バイナリ**: ネイティブ Rust。hosted service や account は不要で、どの端末
  でも即座に開けます。
- **自動化対応**: `kanterm-mcp` がカード・列・ボード・memory log を MCP 経由で公開し、
  claim lease、永続 handoff、検証フィールドで再開可能・監査可能な作業を支えます。
- **実行指向のカード**: handoff note、依存関係（DAG）、実行メタデータ、ボード単位
  の規約により、計画を claim・検証できる作業へ落とし込めます。
- **複数ボード + memory log**: `workflow` / `planning` / `simple` の列テンプレート、
  ボード横断移動、アーカイブと復元、セッションを跨いで残る意思決定・学習ログ。
- **テーマ対応**: 組み込みの `dark` / `light` テーマと JSON による色の上書き。

## クイックスタート

```sh
cargo build --release
./target/release/kanterm          # TUI を起動
```

任意の MCP クライアントから動かす方法は [docs/mcp.ja.md](docs/mcp.ja.md) を
参照してください。

## 仕組み

```
crates/
├─ kanterm-core   ドメイン + SQLite（WAL）。DB に触れる唯一のコード。
├─ kanterm    ratatui ボード。同期的な terminal UI。binary は kanterm。
└─ kanterm-mcp    rmcp stdio MCP サーバ。非同期。binary は kanterm-mcp。
```

データベースの場所は `KANBAN_DB` で変更できます。
設計と各判断の根拠は [DESIGN.ja.md](DESIGN.ja.md) を参照してください。

## 使用方法

### TUI

```sh
./target/release/kanterm
```

`h`/`l` で列移動、`j`/`k` で列内移動、`H`/`L` でカードを列間移動、`Enter` で
カードを開く、`n` で新規作成、`b` でボード切替、`q` で終了。フォーカス中の列・
選択中のカード・アクティブボードは起動間で記憶されます。

全キーバインド、カード詳細モーダル、label picker、テーマ、エクスポート、
バックアップ/復元は **[docs/tui.ja.md](docs/tui.ja.md)** にまとめています。

### MCP

`kanterm-mcp` は stdio 経由でボードを MCP クライアントに公開します。カードは key
（例: `KB-12`）で指定し、ツールは参照（`get_board`・`list_cards`・`get_card`）、
更新（`create_card`・`create_cards`・`update_card`）、構造（`manage_columns`・
`manage_boards`）、連携、永続 handoff、memory log をカバーします。
`kanterm-mcp watch-handoffs` は、永続 handoff を別 runtime に配送する軽量 watcher /
bridge として動かせます。`kanterm-mcp run-workflow` は、小さな workflow YAML の
step 完了を cross-repo handoff に変換できます。再利用可能な target config により、
現時点では command target へ配送でき、interactive session target は terminal
adapter 用に予約されています。

ツールの全リファレンス、実行フロー、実行メタデータ、queue フィルタ、import 例は
**[docs/mcp.ja.md](docs/mcp.ja.md)** にあります。

## ドキュメント

- TUI リファレンス: [docs/tui.ja.md](docs/tui.ja.md)
- MCP リファレンス: [docs/mcp.ja.md](docs/mcp.ja.md)
- 設計と根拠: [DESIGN.ja.md](DESIGN.ja.md) / [DESIGN.md](DESIGN.md)
- 貢献: [CONTRIBUTING.ja.md](CONTRIBUTING.ja.md) / [CONTRIBUTING.md](CONTRIBUTING.md)
- リリース: [RELEASE.ja.md](RELEASE.ja.md) / [RELEASE.md](RELEASE.md)
- セキュリティ: [SECURITY.ja.md](SECURITY.ja.md) / [SECURITY.md](SECURITY.md)
- 変更履歴: [CHANGELOG.ja.md](CHANGELOG.ja.md) / [CHANGELOG.md](CHANGELOG.md)
- ボード移行（MCP）: [docs/mcp-card-migration.ja.md](docs/mcp-card-migration.ja.md) / [docs/mcp-card-migration.en.md](docs/mcp-card-migration.en.md)

## 開発方針

本プロジェクトは個人開発ベースで保守しています。
**Pull Request は受け付けません。** バグ報告、機能要望、質問は GitHub Issues で
受け付けます。

## ライセンス

MIT。[LICENSE](LICENSE) を参照してください。
