# リリースチェックリスト

English version: [RELEASE.md](RELEASE.md)

## OSS 公開前の確認

- 公開リポジトリ URL を確認し、`workspace.package.repository` を更新する。
- `README.md` のインストール手順が公開先に一致しているか確認する。
- README/README.ja、DESIGN/DESIGN.ja、MCP instructions/tool descriptions、
  board `agent_context` の drift を確認する recurring maintenance card を実行または作成する。
  曖昧な cleanup メモではなく、`create_cards` で具体カードに分ける。
- `LICENSE` がリポジトリ直下に存在すること。
- 新しい release binary を実際のローカル board に対して試す前に、repo 外へ timestamp 付き
  backup を取る。

```sh
./target/release/kanterm --backup-db ~/kanban-backups/kanban-$(date +%Y%m%d-%H%M%S).db
```

- 以下を実行する。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release --workspace
cargo package --list -p kanban-core
cargo package --list -p kanban-tui
cargo package --list -p kanban-mcp
cargo package -p kanban-core
cargo package -p kanban-tui --no-verify
cargo package -p kanban-mcp --no-verify
```

CI では上記の clean worktree 前提のコマンドを実行します。`--allow-dirty` は、
コミット前のローカル変更を意図的に含めて package 出力を確認したい場合だけ使ってください。

`kanban-tui` と `kanban-mcp` は crates.io 向けに crate 名を変更するまで `--no-verify`
で tarball 内容を確認します。現状の名前で完全 verify すると、registry 上の無関係な
`kanban-core` crate を解決してしまうためです。この制約は下記の公開ブロッカーです。

## crates.io について

現時点の crate 名は公開には未対応です。

- `kanban-core` は crates.io で既に使用済み
- `kanban-tui` は crates.io で既に使用済み
- `kanban-tui` と `kanban-mcp` は同ワークスペースの `kanban-core` に依存するため、
  そのまま同名公開すると別の crate を解決してしまう可能性がある

公開準備を進める場合は、crate 名の変更と依存定義の更新が必要です。

```sh
cargo package --list -p <crate> --allow-dirty
cargo publish --dry-run -p <crate> --allow-dirty
```
