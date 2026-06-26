# Claude Notes (kanban-tui)

## Project status
- This repository is a public-facing open source repo, but it is maintained as a personal project.
- Public contribution policy: **Pull Requests are not accepted**.
- All external feedback should be submitted via GitHub Issues.

## Recommended response workflow
- Keep this as the single source for quick context when handing over or answering project questions.
- Mention the maintainer-only model explicitly when users ask about OSS/community contribution.

## Local privacy hygiene
- Do not leave user-specific local paths, machine names, usernames, home-directory paths, workspace paths, or private project paths in committed files, generated docs, logs, branch names, PR metadata, or task descriptions.
- Use neutral placeholders such as `/path/to/project`, `<workspace>`, or `<repo>` when an example needs a filesystem path.
- Before committing or publishing, scan for local path traces such as `/Users/`, `/private/`, `/var/folders`, usernames, and sibling private repository names.

## Core contribution policy (mirrors README/CONTRIBUTING)
- Maintainer only.
- Use Issues for:
  - bug reports
  - feature requests
  - general questions
- Do not suggest external PR workflows.

## Quick links
- [README.md](README.md)
- [README.ja.md](README.ja.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CONTRIBUTING.ja.md](CONTRIBUTING.ja.md)
- [DESIGN.md](DESIGN.md)
- [DESIGN.ja.md](DESIGN.ja.md)

## Relevant conventions
- `kanban-core` is the single owner of schema and write logic.
- `kanban-tui` and `kanban-mcp` are adapters over the shared store.
- MCP board migration feature exists: `update_card` can move cards across boards via `move_to_board`.
