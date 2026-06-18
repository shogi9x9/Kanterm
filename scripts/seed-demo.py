#!/usr/bin/env python3
"""Seed a throwaway demo board used to record docs/assets/demo.gif.

Speaks JSON-RPC to the release `kanban-mcp` binary over stdio, then sets the
TUI's restored board so the recording opens straight onto the demo board.

Usage:
    cargo build --release -p kanban-mcp -p kanban-tui
    python3 scripts/seed-demo.py
    vhs docs/assets/demo.tape
"""
import json
import os
import re
import sqlite3
import subprocess
import sys

BIN = "./target/release/kanban-mcp"
DB = "/tmp/kanban-demo.db"
BOARD_SLUG = "project-phoenix"

for path in (DB, DB + "-wal", DB + "-shm"):
    if os.path.exists(path):
        os.remove(path)

env = dict(os.environ, KANBAN_DB=DB)
proc = subprocess.Popen(
    [BIN], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env, text=True, bufsize=1
)

_id = 0


def call(method, params=None, notify=False):
    global _id
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    if not notify:
        _id += 1
        msg["id"] = _id
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()
    if notify:
        return None
    while True:
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError("kanban-mcp closed unexpectedly")
        try:
            resp = json.loads(line)
        except json.JSONDecodeError:
            continue
        if resp.get("id") == _id:
            return resp


def tool(name, args):
    resp = call("tools/call", {"name": name, "arguments": args})
    try:
        return resp["result"]["content"][0]["text"]
    except Exception:
        return json.dumps(resp)


call(
    "initialize",
    {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "seed-demo", "version": "0"},
    },
)
call("notifications/initialized", notify=True)

out = tool(
    "manage_boards",
    {"action": "create", "name": "Project Phoenix", "template": "workflow"},
)
print("board:", out.strip(), file=sys.stderr)

CARDS = [
    ("Todo", "Design import/export schema", 1, ["docs"]),
    ("Todo", "Add CJK width handling to body editor", 2, ["bug"]),
    ("Todo", "Theme: high-contrast variant", 0, ["ui"]),
    ("In progress", "Wire MCP claim leases into TUI header", 2, ["mcp"]),
    ("In progress", "Dependency graph: executable stages", 1, ["mcp", "graph"]),
    ("Testing", "Backup/restore round-trip checks", 1, ["release"]),
    ("Waiting for release", "v0.2 changelog + notes", 0, ["release", "docs"]),
]
for col, title, prio, labels in CARDS:
    created = tool("create_card", {"board": BOARD_SLUG, "column": col, "title": title})
    key = re.search(r"[A-Z]{2,}-\d+", created)
    if key and (prio or labels):
        args = {"board": BOARD_SLUG, "key": key.group(0)}
        if prio:
            args["priority"] = prio
        if labels:
            args["add_labels"] = labels
        tool("update_card", args)
    print("card:", col, "/", title, "->", created.strip(), file=sys.stderr)

tool(
    "manage_boards",
    {
        "action": "set_context",
        "board": BOARD_SLUG,
        "agent_context": "Run cargo fmt + clippy before completing any card.",
    },
)

proc.stdin.close()
proc.wait(timeout=5)

# Open the recording directly on the demo board, with a card pre-selected.
con = sqlite3.connect(DB)
con.executemany(
    "INSERT INTO ui_state(key, value) VALUES(?, ?) "
    "ON CONFLICT(key) DO UPDATE SET value=excluded.value",
    [("tui.board", BOARD_SLUG), ("tui.focus", "1"), ("tui.selected_key", "PP-5")],
)
con.commit()
con.close()

print("seeded ->", DB, file=sys.stderr)
