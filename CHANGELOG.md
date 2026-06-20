# Changelog

日本語版: [CHANGELOG.ja.md](CHANGELOG.ja.md)

All notable changes to this project will be documented in this file.

This project follows a lightweight changelog format and uses semantic versioning
once public releases begin.

## Unreleased

- **Breaking (MCP):** `create_card` and `create_cards` now require `board`.
  Passing an existing project board slug targets that board; passing an unknown
  name creates a new workflow-template board and adds the card(s) there. Omitting
  `board` is now an error instead of silently falling back to Backlog.
- Add `create_card_in_backlog` MCP tool: the Backlog inbox is now opt-in only and
  can no longer be reached through `create_card`/`create_cards`.
- `create_card`/`create_cards` responses now report the destination board slug
  and whether the board already existed or was created.
- Add local TUI kanban board backed by SQLite.
- Add MCP server for AI agents.
- Add memory log with recall tracking and monthly cleanup.
- Add agent workflow fields and advisory card leases.
- Add theme support, board ordering, and planning lanes.
- Add MIT license and release preparation docs.
