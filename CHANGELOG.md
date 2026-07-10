# Changelog

日本語版: [CHANGELOG.ja.md](CHANGELOG.ja.md)

All notable changes to this project will be documented in this file.

This project follows a lightweight changelog format and uses semantic versioning
once public releases begin.

## Unreleased

- Make a cross-board execution dashboard the first TUI view (`W`), covering
  running, human-gated, ready, explicitly blocked, dependency-waiting, and
  missing-context work with direct navigation to card detail and visible
  blocker keys. Add LIST, dependency-stage TIMELINE, and derived-state FLOW
  projections with `Tab` and `1` / `2` / `3` navigation.
- Add the default transparent-background `glass` theme and modernize the TUI
  header, column spacing, selection markers, and responsive key hints.

## 0.1.0 - 2026-07-09

- **Breaking (MCP):** `create_card` and `create_cards` now require `board`.
  Passing an existing project board slug targets that board; passing an unknown
  name creates a new workflow-template board and adds the card(s) there. Omitting
  `board` is now an error instead of silently falling back to Backlog.
- Add `create_card_in_backlog` MCP tool: the Backlog inbox is now opt-in only and
  can no longer be reached through `create_card`/`create_cards`.
- `create_card`/`create_cards` responses now report the destination board slug
  and whether the board already existed or was created.
- Add local TUI kanban board backed by SQLite.
- Add MCP server for automation clients.
- Add memory log with recall tracking and monthly cleanup.
- Add workflow handoff fields and advisory card leases.
- Add theme support, board ordering, and planning lanes.
- Align Rust package names with the project name: `kanterm-core`, `kanterm`,
  and `kanterm-mcp`.
- Add MIT license and release preparation docs.
