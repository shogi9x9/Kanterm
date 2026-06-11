# Release Checklist

日本語版: [RELEASE.ja.md](RELEASE.ja.md)

## Open Source Release

- Confirm the public repository URL and add it to `workspace.package.repository`.
- Review `README.md` for installation instructions that match the public repo.
- Run or create recurring maintenance cards for README/README.ja parity,
  DESIGN/DESIGN.ja parity, MCP instruction/tool description drift, and board
  `agent_context` drift. Use `create_cards`; do not leave vague cleanup notes.
- Keep `LICENSE` in the repository root.
- Before testing the new release binary against your real local board, take a
  timestamped backup outside the repo:

```sh
./target/release/kanban-tui --backup-db ~/kanban-backups/kanban-$(date +%Y%m%d-%H%M%S).db
```

- Run:

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

CI runs the clean-worktree form above. Use `--allow-dirty` only for an explicit
local preflight when you intentionally want to inspect package output before
committing the current workspace changes.

`kanban-tui` and `kanban-mcp` use `--no-verify` until the crates are renamed for
crates.io. Full verification of those package tarballs currently resolves the
registry's unrelated `kanban-core` crate, which is the publication blocker
listed below.

## crates.io

The current crate names are good for local development, but they are not ready
for crates.io publication:

- `kanban-core` is already taken on crates.io.
- `kanban-tui` is already taken on crates.io.
- `kanban-tui` and `kanban-mcp` depend on this workspace's `kanban-core`; if
  published under the current names, Cargo resolves the registry's unrelated
  `kanban-core` instead.

Before publishing to crates.io, choose available package names and update:

- `[package].name` in each crate
- the workspace `kanban-core` dependency name/version
- README install examples, if any

Then run:

```sh
cargo package --list -p <crate> --allow-dirty
cargo publish --dry-run -p <crate> --allow-dirty
```
