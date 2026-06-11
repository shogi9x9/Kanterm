# Security Policy

日本語版: [SECURITY.ja.md](SECURITY.ja.md)

## Supported Versions

This project is pre-1.0. Security fixes are handled on the latest mainline
version.

## Reporting a Vulnerability

If you find a vulnerability, please avoid filing a public issue with exploit
details. Report it privately to the project maintainer through the repository's
private vulnerability reporting channel.

## Scope

kanban-tui is a local-first application. The primary security boundaries are:

- local SQLite database integrity
- MCP tool inputs from local AI agents
- terminal rendering of user-controlled task text

Please include reproduction steps, affected version or commit, and the impact.
