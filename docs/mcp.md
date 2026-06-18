# MCP reference (for agents)

日本語版: [mcp.ja.md](mcp.ja.md)

The repo ships a project-scoped [`.mcp.json`](../.mcp.json) so Claude Code picks
up the `kanban` server automatically when run from this directory.

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
7. Record non-obvious decisions with `record_memory`.

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
      "next_action": "Compare crates/kanban-mcp/src/instructions.rs, tool descriptions, and get_board board_agent_context; create or patch exact drift.",
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
KANBAN_DB=/tmp/k.db ./target/release/kanban-mcp
# then speak JSON-RPC: initialize → notifications/initialized → tools/list
```

## Board migration

- English: [mcp-card-migration.en.md](mcp-card-migration.en.md)
- 日本語: [mcp-card-migration.ja.md](mcp-card-migration.ja.md)
