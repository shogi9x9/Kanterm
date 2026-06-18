#!/usr/bin/env bash
# Build the binaries and seed a throwaway demo board for the README recording.
#
#   scripts/seed-demo.sh
#   vhs docs/assets/demo.tape
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -p kanban-mcp -p kanban-tui
python3 scripts/seed-demo.py
echo "Now run: vhs docs/assets/demo.tape"
