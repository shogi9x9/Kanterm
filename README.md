# Kanterm

> A local-only kanban board for your terminal — with an MCP server so AI agents
> work the same board you do.

[![CI](https://github.com/shogi9x9/Kanterm/actions/workflows/ci.yml/badge.svg)](https://github.com/shogi9x9/Kanterm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

日本語版: [README.ja.md](README.ja.md)

![Kanterm demo](docs/assets/demo.gif)

A single-user, local-first task store with two front ends over one SQLite
database: a **terminal UI** for humans to plan, audit, and intervene, and an
**MCP server** that lets AI agents (Claude, Codex) read and update the very same
cards. Both can run at the same time and see each other's writes live.

## Features

- **One board, two surfaces** — the TUI and the MCP server go through the same
  `kanban-core` crate and the same SQLite (WAL) database, so what you see and
  what an agent edits are always the same data.
- **Local-only, single binary** — native Rust, no server and no account; your
  data lives in a local SQLite file and opens instantly in any terminal.
- **Agent-native** — `kanban-mcp` exposes cards, columns, boards, and a memory
  log to agents, with claim leases, durable handoffs, and verification fields
  for resumable, auditable work.
- **Execution-oriented cards** — handoff notes, dependencies (DAGs), execution
  metadata, and per-board agent instructions turn a plan into claimable,
  verifiable work.
- **Multiple boards + memory log** — `workflow` / `planning` / `simple` column
  templates, cross-board moves, archive & restore, and a durable
  decisions/learnings log that survives across sessions.
- **Themeable** — built-in `dark` / `light` themes plus JSON color overrides.

## Quickstart

```sh
cargo build --release
./target/release/kanterm          # open the TUI
```

The repo ships a project-scoped [`.mcp.json`](.mcp.json), so Claude Code picks up
the `kanterm` MCP server automatically when run from this directory. See
[docs/mcp.md](docs/mcp.md) to drive it from any MCP client.

## How it works

```
crates/
├─ kanban-core   domain + SQLite (WAL). The ONLY code that touches the DB.
├─ kanban-tui    ratatui board, synchronous terminal UI. Binary: kanterm.
└─ kanban-mcp    rmcp stdio MCP server, async, for agents. Binary: kanterm-mcp.
```

Data lives at `~/.local/share/kanban/kanban.db` (override with `KANBAN_DB`).
See [DESIGN.md](DESIGN.md) for the full design and the rationale behind each
decision.

## Usage

### TUI

```sh
./target/release/kanterm
```

`h`/`l` move between columns, `j`/`k` within a column, `H`/`L` move a card across
columns, `Enter` opens a card, `n` adds one, `b` switches boards, `q` quits.
The board remembers your focused column, selected card, and active board between
launches.

Full keybindings, the card detail modal, label picker, themes, export, and
backup/restore are documented in **[docs/tui.md](docs/tui.md)**.

### MCP (for agents)

`kanterm-mcp` exposes the board to AI agents over stdio. Cards are addressed by
key (e.g. `KB-12`); tools cover reading (`get_board`, `list_cards`, `get_card`),
writing (`create_card`, `create_cards`, `update_card`), structure
(`manage_columns`, `manage_boards`), agent coordination (`register_agent`,
claims, `dependency_graph`, durable handoffs), and a memory log
(`record_memory`, `recall_memories`).
`kanterm-mcp watch-handoffs` can run as a lightweight watcher/bridge for
delivering durable handoffs into another local runtime, and
`kanterm-mcp run-workflow` can turn a small workflow YAML step completion into a
cross-repo handoff. Reusable target configs let workflows route to command
targets now, with interactive session targets reserved for terminal adapters.
`kanterm-mcp run-agent-task` closes the headless loop by claiming an incoming
handoff, running the receiving command target, completing a Kanterm card, and
triggering the next workflow step.

The full tool reference, the agent execution flow, execution metadata, queue
filters, and import examples are in **[docs/mcp.md](docs/mcp.md)**.

## Documentation

- TUI reference: [docs/tui.md](docs/tui.md)
- MCP reference: [docs/mcp.md](docs/mcp.md)
- Design & rationale: [DESIGN.md](DESIGN.md) / [DESIGN.ja.md](DESIGN.ja.md)
- Contributing: [CONTRIBUTING.md](CONTRIBUTING.md) / [CONTRIBUTING.ja.md](CONTRIBUTING.ja.md)
- Releases: [RELEASE.md](RELEASE.md) / [RELEASE.ja.md](RELEASE.ja.md)
- Security: [SECURITY.md](SECURITY.md) / [SECURITY.ja.md](SECURITY.ja.md)
- Changelog: [CHANGELOG.md](CHANGELOG.md) / [CHANGELOG.ja.md](CHANGELOG.ja.md)
- Board migration (MCP): [docs/mcp-card-migration.en.md](docs/mcp-card-migration.en.md) / [docs/mcp-card-migration.ja.md](docs/mcp-card-migration.ja.md)

## Project status

This project is primarily maintained by a single maintainer.
**Pull requests are not accepted.** Please use GitHub Issues for bug reports,
enhancement requests, and general questions.

## License

MIT. See [LICENSE](LICENSE).
