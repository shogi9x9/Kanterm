# Contributing

日本語版: [CONTRIBUTING.ja.md](CONTRIBUTING.ja.md)

Thanks for taking the time to improve kanban-tui.

This repository is maintained by a single maintainer.
**Pull requests are not accepted**.
Please use Issues for bug reports, enhancement requests, and general questions.

## Development

Use Rust 1.90 or newer:

```sh
rustup show
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Build release binaries only when needed:

```sh
cargo build --release
```

## Architecture

- `kanban-core` owns all SQLite schema, migrations and domain rules.
- `kanban-tui` owns terminal interaction and should stay synchronous.
- `kanban-mcp` is a thin MCP adapter over `kanban-core`.

Prefer changes that keep database behavior in `kanban-core` and keep the TUI and
MCP surfaces as adapters.

## Pull Requests

Please note: PRs are not accepted.

- Keep changes scoped.
- Add or update tests for schema, storage, MCP behavior, or user-visible flows.
- Run the checks above before sharing changes in the project.
- Update `README.md`, `DESIGN.md`, or `RELEASE.md` when behavior or release
  process changes.
