# MCP reference (for agents)

日本語版: [mcp.ja.md](mcp.ja.md)

The repo ships a project-scoped [`.mcp.json`](../.mcp.json) so Claude Code picks
up the `kanterm` server automatically when run from this directory.

## Tools

Cards are addressed by key, e.g. `KB-12`.

| Tool          | Purpose                                              |
| ------------- | ---------------------------------------------------- |
| `status`      | read-only server status: version, schema, DB path, working directory, and default board |
| `get_board`   | overview: every column and its cards (call first)    |
| `list_cards`  | one line per card, with column/status/query filters; `query` uses the FTS5-backed card search index |
| `get_card`    | a single card in full, including its body            |
| `create_card` | create a card (`title`, optional `body`, `column`)   |
| `create_cards` | create ordered cards from a spec/plan, with optional execution metadata |
| `dependency_graph` | render dependency edges, executable stages, and blockers |
| `register_agent` | request an agent display name and receive an assigned identity such as `codex#abc123` plus a claim token |
| `send_handoff` | send a durable inbox message to an exact agent identity or agent family name |
| `list_handoffs` | list open handoffs, optionally scoped to one agent inbox |
| `claim_handoff` | claim one handoff with an agent identity and claim token, setting a recoverable lease |
| `complete_handoff` | mark a claimed handoff as `completed` or `failed` |
| `update_card` | update any field; `column` moves it, `move_to_board` moves it to another board (by slug), `add_labels` / `remove_labels` tag it, `due` (`YYYY-MM-DD`, `""` clears) sets a deadline, `next_action` / `blocked_reason` / `acceptance_criteria` capture handoff state, execution metadata describes agent suitability, and `claim` / `claim_token` / `release_claim` / `lease_minutes` coordinate agent ownership |
| `manage_columns` | add / rename / delete (with a `to` destination) / reorder columns on project boards; the Backlog board rejects column changes |
| `manage_boards` | create (`name`, optional `template` defaulting to `workflow`, optional `agent_context`) / archive / unarchive / reorder a board by slug; `set_context` / `clear_context` stores board-level agent instructions; `Backlog` is reserved, and `delete` only works on archived project boards |
| `record_memory` | record a decision/learning/context note that survives across sessions; optional `card` link (e.g. `KB-12`) |
| `recall_memories` | search memories by `query`/`card`/`kind`, newest first; `key` (e.g. `M-3`) reads one in full |

Every card/column tool takes an optional `board` slug (defaults to the Backlog
board, whose slug is `backlog`); `get_board` lists all boards at the bottom so
agents can discover and target them. Use `move_to_board` to move cards from the
Backlog board into a project board when they are ready to schedule; the TUI
labels this Backlog action as `send-project`.
Cross-board moves are recorded in card activity with the old key, new key,
source board and destination board so reissued keys remain traceable in
`get_card`.

Labels are created on demand the first time you reference them by name and are
shown inline in `get_board` / `list_cards` and on `get_card`. Due dates render as
`due:YYYY-MM-DD`, prefixed with `!` (and `(overdue)` on `get_card`) when past.

Agents should call `register_agent` before claiming work. Use the returned
`assigned_identity` as `update_card.claim` and pass the returned `claim_token`
when claiming, renewing, or releasing a claim. Active agent leases render as
`[claimed:codex#abc123]`; expired leases render as `[claim-expired:name]` and
can be taken over by another registered agent.
`complete_note` archives the card, appends the note when non-empty, marks
`agent_state=done`, clears active handoff fields, and releases any claim.

Durable agent-to-agent handoffs are separate from card fields. `send_handoff`
stores an inbox item in the same SQLite database and can address either an exact
identity (`claude#abc123`) or an agent family (`claude`). A receiving agent, hook,
or watcher can call `list_handoffs(for_agent="claude#abc123")`, claim work with
`claim_handoff`, then close it with `complete_handoff`. The lease lets another
watcher recover a handoff if the first receiver exits before acting on it.
Runtime-specific hooks or bridges should be thin delivery layers on top of this
queue; the handoff state itself is durable in Kanterm.

`kanterm-mcp` also ships a headless watcher for those delivery layers:

```sh
kanterm-mcp watch-handoffs \
  --for-agent claude#abc123 \
  --claim-token "$CLAIM_TOKEN"
```

The watcher polls the Kanterm DB, claims matching handoffs, writes each claimed
handoff as one JSON line to stdout, and marks it `completed` after successful
delivery. `--once` performs a single scan for hooks/tests. `--interval-ms`
changes the polling interval.
Inspired by agmsg's monitor watcher, each watcher writes a pidfile and ready
sentinel under `KANTERM_RUN_DIR` or the default temp run directory. The ready
file is `watch.<agent>.ready`, and it appears after the watcher has claimed its
slot and completed its first scan. Starting a second watcher for the same agent
fails while the first process is alive; pass `--replace-existing` to replace it.
Use `--skip-if-running` for Stop-hook fallback scans that should quietly exit
when the monitor watcher is already alive. Use `--run-dir DIR` when a supervisor
needs a stable path to wait on.

To bridge into another runtime, pass a command:

```sh
kanterm-mcp watch-handoffs \
  --for-agent claude#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --run-dir /tmp/kanterm-run \
  --bridge-command ./scripts/kanterm-bridge-file-inbox.sh \
  --bridge-arg --repo \
  --bridge-arg /path/to/downstream-repo
```

The bridge receives the handoff body on stdin and metadata in environment
variables such as `KANTERM_HANDOFF_ID`, `KANTERM_HANDOFF_FROM_AGENT`,
`KANTERM_HANDOFF_TO_AGENT`, `KANTERM_HANDOFF_SUBJECT`, `KANTERM_HANDOFF_CARD_KEY`,
and `KANTERM_HANDOFF_LEASE_EXPIRES_AT`. Exit code 0 marks the handoff
`completed`; a non-zero exit marks it `failed` with the bridge error.
Kanterm ships two generic bridges: `scripts/kanterm-bridge-file-inbox.sh` writes
a Markdown inbox file under the target repo, and
`scripts/kanterm-bridge-agent-command.sh` runs an arbitrary command in the target
repo with a formatted handoff prompt on stdin.

For repeatable routing, put delivery targets in a small YAML config and pass
`--targets PATH --target NAME` instead of repeating bridge arguments:

```yaml
targets:
  - name: bff-command
    type: command
    agent: bff-agent
    repo: /path/to/downstream-repo
    command: claude
    args: -p

  - name: bff-session
    type: interactive
    agent: bff-agent
    adapter: tmux
    session: bff
    pane: 0
```

```sh
kanterm-mcp watch-handoffs \
  --for-agent bff-agent#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --targets ./kanterm.targets.yaml \
  --target bff-command
```

`type: command` is implemented now: Kanterm starts `command` with `args` in the
target `repo` and writes a formatted handoff prompt to stdin. `type:
interactive` is a reserved target shape for tmux/zellij session adapters; it is
validated by config parsing but watcher delivery returns an explicit
not-implemented error until those adapters are added.

For cross-repo orchestration, `kanterm-mcp run-workflow` can turn a workflow step
completion into a durable handoff. This is intentionally a small YAML subset: it
supports named steps and an `on_complete.send_handoff` action, then relies on the
same watcher/bridge layer for delivery.

```yaml
name: ms-to-bff
initial_step: implement_ms
steps:
  - name: implement_ms
    agent: ms-agent
    on_complete:
      send_handoff:
        target: bff-command
        subject: Continue {{card}} for {{workflow}}
        body: Continue {{step}} from {{from_agent}} into {{repo}}
```

```sh
kanterm-mcp run-workflow \
  --workflow ./kanterm.workflow.yaml \
  --targets ./kanterm.targets.yaml \
  --from-agent ms-agent \
  --board ms \
  --card MS-1
```

The runner renders `{{workflow}}`, `{{step}}`, `{{step_agent}}`,
`{{from_agent}}`, `{{target}}`, `{{to_agent}}`, `{{repo}}`, `{{board}}`, and
`{{card}}`. `send_handoff.to_agent` can still be written directly; when
`send_handoff.target` is used, the recipient comes from the target's `agent`
field or falls back to the target name. When `--board` and `--card` are
supplied, the handoff is linked to that Kanterm card and normal card validation
applies.

The same workflow can be triggered directly from `update_card` when completing a
card. Workflow trigger fields are accepted only with `complete_note`, so normal
edits cannot accidentally enqueue downstream work:

```json
{
  "board": "ms",
  "key": "MS-1",
  "complete_note": "implemented and verified",
  "workflow": "./kanterm.workflow.yaml",
  "workflow_targets": "./kanterm.targets.yaml",
  "workflow_from_agent": "ms-agent"
}
```

On success, the `update_card` response includes `workflow_triggered:` followed by
the same summary as `run-workflow`. The trigger uses the completed card's board
and key, so workflow templates can use `{{board}}` and `{{card}}` without
duplicating those values.

To let the receiving side continue the chain without an outer script manually
calling `update_card`, use `kanterm-mcp run-agent-task`. It claims one incoming
handoff, runs the configured command target, completes the specified Kanterm
card with the command output as the completion note, and optionally triggers the
next workflow step:

```sh
kanterm-mcp run-agent-task \
  --for-agent b#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --targets ./kanterm.targets.yaml \
  --target b-command \
  --board ms \
  --card MS-2 \
  --workflow ./kanterm.workflow.yaml \
  --workflow-targets ./kanterm.targets.yaml \
  --workflow-step b-to-c \
  --from-agent b
```

This runner is the headless command-agent loop: watcher-style claim semantics on
the incoming handoff, command-target execution for the receiving agent, Kanterm
card completion, then workflow-triggered handoff to the next agent.

For Claude Code, Kanterm can install project-local hooks in
`.claude/settings.local.json`:

```sh
kanterm-mcp hooks install \
  --runtime claude-code \
  --mode both \
  --for-agent claude#abc123 \
  --claim-token "$CLAIM_TOKEN" \
  --run-dir /tmp/kanterm-run \
  --bridge-command ./scripts/kanterm-bridge-file-inbox.sh \
  --bridge-arg --repo \
  --bridge-arg /path/to/downstream-repo
```

The installer is idempotent: it removes prior Kanterm-owned `SessionStart`,
`SessionEnd`, and `Stop` entries before adding the entries for the selected
mode. Unowned hooks are preserved. Modes mirror agmsg's delivery split:
`monitor` installs `SessionStart`/`SessionEnd` background watcher hooks, `turn`
installs a `Stop` fallback that runs one scan between turns, `both` installs
both; the `Stop` fallback uses `--skip-if-running` so it does not race the live
monitor watcher. `off` strips Kanterm-owned hooks. Use `kanterm-mcp hooks status`
to inspect the current mode and `kanterm-mcp hooks uninstall` to remove the hooks.

Boards can also carry `agent_context`: project-level instructions such as
verification commands, completion policy, repo conventions, or release gates.
Set it with `manage_boards(action="set_context", board="slug",
agent_context="...")`, or pass `agent_context` while creating a board.
`get_board` and `get_card` include it when present so agents see board rules
before executing individual card `next_action` values.

## Agent execution flow

A typical agent workflow is:

1. Turn a spec or plan into durable cards with `create_cards`.
2. Include `alias` and `depends_on` when the plan is a DAG.
3. Put execution context on each card: `acceptance_criteria`, `next_action`,
   and optional execution metadata.
4. Inspect stages with `dependency_graph`, then ask for work with
   `list_cards(queue="executable")`.
5. Inspect `get_card`, then `register_agent` and claim the card.
6. Execute, update `last_verification`, then finish with `complete_note`.
7. Use `send_handoff` when another agent or project should continue work.
8. Record non-obvious decisions with `record_memory`.

`priority` remains the human/business priority marker (`[L]`, `[M]`, `[H]`).
Execution metadata is separate:

- `agent_weight`: agent suitability/cost on a small `1..5` scale.
- `agent_effort`: requested reasoning/runtime level, such as `low`, `medium`,
  or `high-reasoning`.
- `suggested_model`: model/profile hint for the task.
- `expected_tokens`: expected token budget.
- `human_intervention`: `none`, `review`, `decision`, or `execution`.

Queue filters keep autonomous work separate from human-gated work:
`queue=executable`, `queue=review`, `queue=blocked`, `queue=claimed`,
`queue=missing_context`, `queue=dependency_blocked`, and `queue=human`.
Pass `ranked=true` to sort matching cards by next-work suitability and include
compact rank reasons.
Dependency graphs are first-class data; `dependency_graph` renders explicit
edges and executable stages, such as `A -> B/C/D in parallel -> E`. Use
`active_only=true` to hide closed historical edges, or `focus="KEY"` to inspect
one card and its direct neighbours.

## Import examples

Minimal fan-out/fan-in import:

```json
{
  "board": "plan",
  "cards": [
    { "alias": "A", "title": "A", "acceptance_criteria": "A done", "next_action": "do A" },
    { "alias": "B", "title": "B", "depends_on": ["A"], "acceptance_criteria": "B done", "next_action": "do B" },
    { "alias": "C", "title": "C", "depends_on": ["A"], "acceptance_criteria": "C done", "next_action": "do C" },
    { "alias": "D", "title": "D", "depends_on": ["A"], "acceptance_criteria": "D done", "next_action": "do D" },
    { "alias": "E", "title": "E", "depends_on": ["B", "C", "D"], "acceptance_criteria": "E done", "next_action": "do E" }
  ]
}
```

Recurring maintenance should use the same durable-card flow. Do not leave a
single vague "clean up docs" note; create concrete cards that can be claimed and
verified independently. A useful local pattern is:

```json
{
  "board": "kanban-improvements",
  "cards": [
    {
      "alias": "refactor-scan",
      "title": "Maintenance: scan refactor pressure",
      "acceptance_criteria": "Oversized modules, duplicated policy, or stale helpers are either recorded as concrete follow-up cards or explicitly judged acceptable.",
      "next_action": "Compare current diff and module boundaries, then create focused follow-up cards for real refactor pressure.",
      "agent_weight": 2,
      "agent_effort": "medium",
      "expected_tokens": 2500
    },
    {
      "alias": "readme-parity",
      "title": "Maintenance: README English/Japanese parity",
      "acceptance_criteria": "README.md and README.ja.md describe the same public behavior, tool names, and release caveats.",
      "next_action": "Diff README.md against README.ja.md and patch only behavior drift.",
      "agent_weight": 1,
      "agent_effort": "low",
      "expected_tokens": 1500
    },
    {
      "alias": "design-parity",
      "title": "Maintenance: DESIGN English/Japanese parity",
      "acceptance_criteria": "DESIGN.md and DESIGN.ja.md reflect the same architecture decisions and shipped behavior.",
      "next_action": "Compare design docs against current core/MCP/TUI behavior and patch drift.",
      "agent_weight": 1,
      "agent_effort": "low",
      "expected_tokens": 1500
    },
    {
      "alias": "agent-surface-drift",
      "title": "Maintenance: MCP instructions and board context drift",
      "acceptance_criteria": "MCP server instructions, tool descriptions, and board agent_context match the current agent execution flow.",
      "next_action": "Compare crates/kanterm-mcp/src/instructions.rs, tool descriptions, and get_board board_agent_context; create or patch exact drift.",
      "agent_weight": 2,
      "agent_effort": "medium",
      "expected_tokens": 2500
    }
  ]
}
```

Concrete metadata examples:

- Broad refactor: high `agent_weight`, high `agent_effort`, explicit
  `acceptance_criteria`, maybe `human_intervention=review`.
- UI judgment: `human_intervention=decision` or `review`.
- Docs cleanup: low weight, low effort, small token budget.
- Ambiguous product decision: `human_intervention=decision`.
- High-token research: high `expected_tokens`, suggested stronger model.
- Mechanical edit: low weight, low effort, `human_intervention=none`.

## Quick manual check over stdio

```sh
KANBAN_DB=/tmp/k.db ./target/release/kanterm-mcp
# then speak JSON-RPC: initialize → notifications/initialized → tools/list
```

## Board migration

- English: [mcp-card-migration.en.md](mcp-card-migration.en.md)
- 日本語: [mcp-card-migration.ja.md](mcp-card-migration.ja.md)
