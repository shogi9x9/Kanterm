# Release Checklist

日本語版: [RELEASE.ja.md](RELEASE.ja.md)

## Open Source Release

- Confirm the public repository URL and add it to `workspace.package.repository`.
- Review `README.md` for installation instructions that match the public repo.
- Run or create recurring maintenance cards for README/README.ja parity,
  DESIGN/DESIGN.ja parity, MCP instruction/tool description drift, and board
  context drift. Use `create_cards`; do not leave vague cleanup notes.
- Keep `LICENSE` in the repository root.
- Before testing the new release binary against an existing board, take a
  timestamped backup outside the repo:

```sh
./target/release/kanterm --backup-db <backup-file>
```

- Run:

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

CI runs the clean-worktree form above. Use `--allow-dirty` only for an explicit
local preflight when you intentionally want to inspect package output before
committing the current workspace changes.

`kanterm` and `kanterm-mcp` can list package contents locally, but full package
creation waits until `kanterm-core` is published and resolvable from crates.io.
Cargo checks registry availability for packaged path dependencies even when
verification is skipped.

## crates.io

The package names are aligned with the project name:

- `kanterm-core`
- `kanterm`
- `kanterm-mcp`

Before publishing to crates.io, verify name availability and publish in
dependency order:

1. `kanterm-core`
2. `kanterm`
3. `kanterm-mcp`

Then run:

```sh
cargo publish --dry-run -p <crate>
```

## GitHub Release

GitHub Releases are created by pushing a `v*` tag. For `0.1.0`:

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

The release workflow uploads:

- `kanterm-linux-x86_64.tar.gz`
- `kanterm-macos-arm64.tar.gz`
- `SHA256SUMS`
