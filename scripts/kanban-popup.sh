#!/usr/bin/env bash
# Launch the kanban TUI. Can be run directly or from an optional tmux binding.
# Builds on first run if the release binary is missing.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$REPO/target/release/kanban-tui"

if [[ ! -x "$BIN" ]]; then
    echo "Building kanban-tui (first run)…" >&2
    (cd "$REPO" && cargo build --release -p kanban-tui)
fi

exec "$BIN"
