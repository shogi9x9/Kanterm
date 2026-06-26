#!/usr/bin/env python3
"""Seed a throwaway demo board used to record docs/assets/demo.gif.

Speaks JSON-RPC to the release `kanterm-mcp` binary over stdio, then sets the
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

BIN = "./target/release/kanterm-mcp"
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
            raise RuntimeError("kanterm-mcp closed unexpectedly")
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

# alias -> (column, title, priority, labels, extra update_card fields).
# `extra` carries dependencies and agent execution metadata so the dependency
# graph (`g`) and the agent metadata panel (detail -> `m`) have something to show.
CARDS = [
    ("A", "Todo", "Design import/export schema", 1, ["docs"], {
        "agent_weight": 1, "agent_effort": "low", "expected_tokens": 1500,
        "next_action": "Draft the JSON/Markdown schema and review fields.",
        "acceptance_criteria": "Schema covers cards, columns, labels, and metadata.",
    }),
    ("B", "Todo", "Add CJK width handling to body editor", 2, ["bug"], {
        "agent_weight": 3, "agent_effort": "medium", "expected_tokens": 4000,
        "human_intervention": "review",
        "next_action": "Measure display width per grapheme and fix cursor math.",
        "acceptance_criteria": "Mixed CJK/ASCII lines render with a correct cursor.",
    }),
    ("C", "Todo", "Theme: high-contrast variant", 0, ["ui"], {
        "agent_weight": 2, "agent_effort": "low", "expected_tokens": 2000,
        "human_intervention": "decision",
    }),
    ("D", "In progress", "Wire MCP claim leases into TUI header", 2, ["mcp"], {
        "agent_weight": 3, "agent_effort": "medium", "expected_tokens": 5000,
        "depends_on": ["A"],
        "next_action": "Render [claimed:...] / [claim-expired:...] in the header.",
        "acceptance_criteria": "Active and expired leases are visible in the TUI.",
    }),
    ("E", "In progress", "Dependency graph: executable stages", 1, ["mcp", "graph"], {
        "agent_weight": 4, "agent_effort": "high-reasoning",
        "suggested_model": "claude-opus", "expected_tokens": 8000,
        "human_intervention": "review", "depends_on": ["A"],
        "next_action": "Compute executable stages and render A -> B/C -> D.",
        "acceptance_criteria": "Graph shows edges, stages, and blocked cards.",
    }),
    ("F", "Testing", "Backup/restore round-trip checks", 1, ["release"], {
        "agent_weight": 2, "agent_effort": "medium", "expected_tokens": 3000,
        "depends_on": ["D", "E"],
        "next_action": "Add a VACUUM INTO round-trip test with schema guard.",
        "acceptance_criteria": "Backup then restore reproduces the board exactly.",
    }),
    ("G", "Waiting for release", "v0.2 changelog + notes", 0, ["release", "docs"], {
        "agent_weight": 1, "agent_effort": "low", "expected_tokens": 1200,
        "depends_on": ["F"],
    }),
]

alias_to_key = {}
for alias, col, title, prio, labels, _extra in CARDS:
    created = tool("create_card", {"board": BOARD_SLUG, "column": col, "title": title})
    key = re.search(r"[A-Z]{2,}-\d+", created)
    if key:
        alias_to_key[alias] = key.group(0)
        args = {"board": BOARD_SLUG, "key": key.group(0)}
        if prio:
            args["priority"] = prio
        if labels:
            args["add_labels"] = labels
        tool("update_card", args)
    print("card:", col, "/", title, "->", created.strip(), file=sys.stderr)

# Second pass: dependencies (resolved to real keys) and execution metadata.
for alias, _col, _title, _prio, _labels, extra in CARDS:
    if not extra or alias not in alias_to_key:
        continue
    args = {"board": BOARD_SLUG, "key": alias_to_key[alias]}
    for field, value in extra.items():
        if field == "depends_on":
            args["depends_on"] = [alias_to_key[a] for a in value if a in alias_to_key]
        else:
            args[field] = value
    tool("update_card", args)

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
