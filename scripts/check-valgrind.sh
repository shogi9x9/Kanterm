#!/usr/bin/env bash
set -euo pipefail

if ! command -v valgrind >/dev/null 2>&1; then
  echo "valgrind is required. Install it with your system package manager." >&2
  exit 127
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

echo "building kanban-core tests"
cargo test -p kanban-core --tests --lib --no-run --message-format=json \
  > "$tmpdir/cargo-test.jsonl"

python3 - "$tmpdir/cargo-test.jsonl" "$tmpdir/test-binaries.txt" <<'PY'
import json
import pathlib
import sys

source = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
executables = []

for line in source.read_text().splitlines():
    if not line.strip():
        continue
    message = json.loads(line)
    if message.get("reason") != "compiler-artifact":
        continue
    executable = message.get("executable")
    if executable:
        executables.append(executable)

destination.write_text("\n".join(dict.fromkeys(executables)) + "\n")
PY

if [[ ! -s "$tmpdir/test-binaries.txt" ]]; then
  echo "no kanban-core test binaries were produced" >&2
  exit 1
fi

while IFS= read -r test_binary; do
  [[ -n "$test_binary" ]] || continue
  echo "valgrind $test_binary"
  valgrind \
    --leak-check=full \
    --show-leak-kinds=definite,possible \
    --errors-for-leak-kinds=definite,possible \
    --error-exitcode=99 \
    "$test_binary"
done < "$tmpdir/test-binaries.txt"
