# リリースチェックリスト

English version: [RELEASE.md](RELEASE.md)

## OSS 公開前の確認

- 公開リポジトリ URL を確認し、`workspace.package.repository` を更新する。
- `README.md` のインストール手順が公開先に一致しているか確認する。
- README/README.ja、DESIGN/DESIGN.ja、MCP instructions/tool descriptions、
  board context の drift を確認する recurring maintenance card を実行または作成する。
  曖昧な cleanup メモではなく、`create_cards` で具体カードに分ける。
- `LICENSE` がリポジトリ直下に存在すること。
- 新しい release binary を既存 board に対して試す前に、repo 外へ backup を取る。

```sh
./target/release/kanterm --backup-db <backup-file>
```

- 以下を実行する。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release --workspace
cargo package --list -p kanterm-core
cargo package --list -p kanterm
cargo package --list -p kanterm-mcp
cargo package -p kanterm-core
```

CI では上記の clean worktree 前提のコマンドを実行します。`--allow-dirty` は、
コミット前のローカル変更を意図的に含めて package 出力を確認したい場合だけ使ってください。

`kanterm` と `kanterm-mcp` はローカルで package contents を確認できますが、package 作成は
`kanterm-core` が crates.io に公開されて解決可能になってから行います。Cargo は
verification を skip しても、packaged path dependency の registry availability を確認します。

## crates.io について

package 名は project 名に合わせています。

- `kanterm-core`
- `kanterm`
- `kanterm-mcp`

crates.io に公開する前に name availability を確認し、依存順に publish します。

1. `kanterm-core`
2. `kanterm`
3. `kanterm-mcp`

```sh
cargo publish --dry-run -p <crate>
```

## GitHub Release

GitHub Releases は `v*` tag の push で作成します。`0.1.0` の場合:

```sh
git switch main
git pull --ff-only
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release -p kanterm -p kanterm-mcp
git tag -a v0.1.0 -m "v0.1.0"
git push origin main
git push origin v0.1.0
```

release workflow は以下を upload します。

- `kanterm-linux-x86_64.tar.gz`
- `kanterm-macos-arm64.tar.gz`
- `SHA256SUMS`
