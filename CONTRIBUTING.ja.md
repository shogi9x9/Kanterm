# Contributing

kanterm の改善にご協力いただき、ありがとうございます。

本リポジトリはシングルメンテナーで運用しています。
**Pull Request は受け付けていません。**
バグ報告・機能要望・質問は Issue で受け付けます。

## PR ポリシー

- 重要な報告は Issue から依頼ください。
- 変更は必要最小限に。
- スキーマ変更、ストレージ変更、MCP の挙動、または利用者向けフローは
  必ずテストを追加・更新する。
- 変更共有前に、上記の開発チェックを実行する。
- 挙動変更やリリース手順変更があれば `README.md` / `DESIGN.md` / `RELEASE.md`
  を更新する。
## 開発環境

以下の前提で開発してください。

- Rust 1.90 以上

```sh
rustup show
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

ストレージ、マイグレーション、低レイヤーのドメイン挙動を変更した場合は、
Linux 環境で Valgrind によるリークチェックも実行してください。

```sh
scripts/check-valgrind.sh
```

このスクリプトは `valgrind` を必要とし、コンパイル済みの `kanterm-core`
テストバイナリを実行して definite / possible leak を失敗扱いにします。

リリース用バイナリは必要な時のみ作成します。

```sh
cargo build --release
```

## アーキテクチャ方針

- `kanterm-core` は SQLite のスキーマ、マイグレーション、ドメインルールを一元管理します。
- `kanterm` は対話系 UI を担当し、同期的に実装します。
- `kanterm-mcp` は `kanterm-core` の薄い MCP アダプタです。

データベースのロジックは `kanterm-core` に集約し、TUI と MCP は
その上位アダプタとして分離して保守します。

## 開発ポリシー

- 変更範囲を必要最小限に絞る。
- スキーマ変更、ストレージ変更、MCP の挙動、または利用者向けフローは
  必ずテストを追加・更新する。
- 変更共有前は上記の開発チェックを実行する。
- 挙動変更やリリース手順変更があれば `README.md` / `DESIGN.md` / `RELEASE.md`
  を更新する。
