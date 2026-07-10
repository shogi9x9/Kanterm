# Design

日本語版: [DESIGN.ja.md](DESIGN.ja.md)

Confirmed design for a local-only task execution store with a terminal UI and
an MCP server for agents. The TUI is a human audit/control panel; MCP is the
agent execution surface. Decisions below were reached jointly with Codex and
Fable (two independent design reviews) and reflect their combined guidance.

## Goals & constraints

- **Local-only, single user.** No cloud, no server, no auth.
- **Terminal ergonomics.** Runs directly in a normal terminal and can also be
  launched from optional wrappers such as tmux popup bindings. Startup latency
  must be negligible because it is opened and closed many times a day.
- **Shared truth with agents.** The TUI a human sees and the board an agent edits
  must be the *same* data, editable concurrently.
- **Durable execution state.** Agent handoff context, execution metadata,
  claims, verification, board-level execution instructions and memories should
  survive process restarts and context compaction.

## Stack

| Concern        | Choice                    | Why                                                |
| -------------- | ------------------------- | -------------------------------------------------- |
| Language / TUI | Rust + ratatui            | single native binary, fast cold start in any terminal |
| Persistence    | SQLite (WAL) via rusqlite | concurrent reader + writer, transactional, queryable |
| MCP            | rmcp (stdio)              | official Rust SDK; keeps everything in one language |

Node/Python were rejected for the TUI: runtime startup cost hurts a local tool
that opens constantly. Go was viable but not installed; Rust was.

## Crate layout

```
kanterm-core   domain types + SQLite. The single owner of the schema and all
              write logic. Nothing else opens the database.
kanterm    ratatui frontend. SYNCHRONOUS (no tokio) — rusqlite is fast and
              local; async would add complexity for no gain.
kanterm-mcp    rmcp stdio server. ASYNC (tokio, required by rmcp). A thin shell
              that locks a Store behind a Mutex and calls core.
```

This boundary means schema and business rules are never implemented twice, and
the MCP surface (the volatile dependency) is isolated to one small crate.

## Concurrency model

The TUI (short-lived) and the MCP server (longer-lived) may write at the same
time. Handled entirely in `kanterm-core::Store::configure` / write paths:

- `journal_mode = WAL` — readers never block the single writer.
- `busy_timeout = 5000ms` — wait, don't immediately fail, on lock contention.
- **`BEGIN IMMEDIATE`** for every write transaction. rusqlite defaults to
  `DEFERRED`, under which a read→write upgrade can hit `SQLITE_BUSY` *ignoring*
  `busy_timeout`. Taking the write lock up front avoids that classic trap.
- `foreign_keys = ON` per connection (SQLite defaults OFF) so cascading deletes
  and referential integrity actually hold.

Live refresh: the TUI's event loop uses `event::poll(800ms)` (an OS timed wait,
not a busy loop). On each idle timeout it compares SQLite's `PRAGMA data_version`
— which changes only when **another** connection commits — and reloads only when
it moved. So an agent's MCP write shows up in the open TUI within ≤800ms, while
an idle board costs one O(1) PRAGMA per interval and zero rendering. Measured
idle cost: 0.0% CPU, ~0.01s cumulative CPU over 8s, single thread, ~3.6MB RSS.
Manual `r` still forces an immediate reload.

## Schema & migrations

- Migrations are an ordered array of `.sql` files gated by
  `PRAGMA user_version` — no migration framework (per review, not worth a dep).
- On open, anything below the target version is applied in an `IMMEDIATE` tx.
- **Forward-incompatibility guard:** if `user_version` is *newer* than the
  binary, open fails; and every write re-checks the version (`assert_writable`)
  so a long-running MCP server that was upgraded underneath won't write with
  stale logic.

Tables: `boards`, `columns`, `cards`, `labels`, `card_labels`, `activity_logs`,
`ui_state`. Two decisions called out by review and adopted up front because
they are painful to retrofit:

- `cards.position REAL` — fractional indexing. Moves are O(1) and don't renumber
  siblings, which plays well with concurrent writers.
- `cards.updated_at` — an optimistic-concurrency anchor for later (v2).

`activity_logs` carries an `actor` column (`tui` vs `agent`) — valuable
precisely because humans and agents share the board. Logs are written in the
application layer, not via triggers (triggers can't see the actor and become a
migration liability).
Cross-board moves add a structured `move_board` activity payload with old/new
keys and source/destination boards, so a card remains traceable when it receives
a new key on the destination board.

`card_search` is an FTS5 virtual table synchronized by the core write path.
`list_cards(query=...)` searches title, body, labels and agent workflow fields
through that index while retaining literal substring fallback behavior for
short or punctuation-heavy queries.

## MCP surface

Per review, kept deliberately compact — fewer tools means higher agent accuracy:

- Core card tools: `get_board`, `list_cards`, `get_card`, `create_card`,
  `create_cards`, `update_card`. `move` is `update_card` with a `column`,
  cross-board move is `update_card` with `move_to_board`, card search is
  `list_cards` with a FTS5-backed `query`, and next-work selection is
  `list_cards(queue=...)`.
- Agent workflow tools: `register_agent`, `record_memory`, `recall_memories`,
  plus board/column management tools.
- Board management includes `agent_context`, a board/project-level instruction
  block for verification commands, completion policy, repo conventions and
  release gates. It deliberately sits above per-card `next_action` so agents can
  load project rules once and apply them to every executable card.
- **No MCP resources** — client support is uneven; reads are tools too, which
  makes agent behaviour predictable.
- **Key-addressed** (`KB-12`); internal ids never leave core.
- **Text/Markdown return values**, not JSON — cheaper for agents to read.

## Shipped baseline

**In:** protected Backlog board plus project boards, configurable project
columns, card create / edit / move / archive / complete, cross-board moves, key
allocation, fractional positions, `user_version` migrations, activity log with
actor, MCP tools for cards/boards/columns/memories/agent registration, and
UI-state restore.

## v2 (shipped)

- **Card detail modal** (`Enter`) and a **multi-line body editor** (`b`,
  `Ctrl-S` to save) in the TUI.
- **In-column reorder** (`J`/`K`) by swapping fractional `position` values.
- **Priority** cycling (`p`, marker on the board) and **assignee** editing (`a`).
- **Labels** end to end: created on demand, colour derived from the name,
  rendered as chips in the TUI and inline in every MCP read. Exposed through
  `update_card`'s `add_labels` / `remove_labels` — **no new MCP tool**, so the
  surface stays at 5 (per the "don't grow the tool list" rule).
- **Export**: `kanterm --export json|md` for git-tracked backups.

## v3 (shipped)

- **Due dates**: `cards.due_date` is set via `update_card`'s `due` field
  (`YYYY-MM-DD`, `""` clears). Dates are parsed/formatted in `kanterm-core`
  with Hinnant's `days_from_civil` / `civil_from_days` — no date-crate
  dependency. Overdue (`due < today_start_ms()`) is flagged in the TUI (red
  `⏰` chip / detail line) and in every MCP read (`!` / `(overdue)`).
- **Search/filter** in the TUI (`/`): narrows every column by a case-insensitive
  substring over title, body and label names. The MCP side already had
  `list_cards`'s `query`.

## v4 (shipped)

- **Multiple boards**: `create_board(name, template)` derives a unique slug and
  an ASCII key prefix (e.g. "Work Stuff" → slug `work-stuff`, keys `WS-1`), then
  creates columns from a named template. Built-ins are `workflow` (the default
  for developer project boards), `planning`, and `simple`. `list_boards` /
  `delete_board` round it out. Keys, columns and cards are namespaced per board,
  so boards are fully isolated (cascade-deleted via `ON DELETE CASCADE`).
  `Backlog` is a reserved system board name; project boards cannot reuse it.
- **TUI**: a header row with a compact active-board selector, `Tab` to switch,
  `b` to choose from a board list, `J/K` in that list to reorder boards, and
  `N` to create by entering a name and choosing a column template. `M` on a
  selected card opens destination board and column pickers, then moves the card
  across boards through the same core update path as MCP. On the protected
  Backlog board, the same action is labelled
  `send-project` so inbox triage reads as scheduling work into a project board.
  `x` completes with an optional note, archives the card, marks it done, clears
  active handoff fields and releases any claim.
  The active board is persisted in `ui_state` and restored.
- **MCP**: every board/card/column tool accepts an optional `board` slug
  (default = Backlog board); `get_board` appends a board directory so agents can
  discover targets. The Backlog board is a one-column inbox/planning board;
  project boards default to the `workflow` template, with `planning` and
  `simple` available when explicitly requested.
- **Agent identity**: agents self-register with `register_agent`, requesting a
  readable name such as `codex`. The core assigns an identity such as
  `codex#abc123`, stores only a hash of the claim token, and requires that token
  for claim, renew and release operations. The assigned identity is also the
  future routing key for agent inbox notifications.

## Testing

- `kanterm-core`: unit tests in-crate plus `tests/integration.rs` —
  migration idempotency/persistence, **concurrent writers** (two threads, unique
  keys under WAL + `BEGIN IMMEDIATE`), reorder edges, cross-column move, board
  isolation/slug/prefix/cascade, clean errors.
- `kanterm-mcp`: `tests/stdio.rs` spawns the real binary and drives JSON-RPC over
  stdio — the five-tool surface, the create/update/move/label flow, due/overdue
  and error handling, and multi-board addressing.

## v5 (shipped)

- **Column templates** replaced a single hard-coded project-board default.
  Built-ins: `workflow` = **Todo / In progress / Testing / Waiting for release**
  and is the default for developer work, `planning` = **Backlog / Today / This
  week / This month**, `simple` = **Todo / Doing / Done**. Existing boards keep
  their columns.
- **Popup editors**: single-line inputs (new card/board, title, assignee, due,
  filter) now render as centered modals instead of a status-bar field.
- **Label picker** (`t` in the detail modal): attach multiple labels, reuse
  existing ones via a `Space`-toggle list, or type to create a new one. Labels
  carry a `last_used_at` (migration 0002, bumped on every attach); the picker
  only suggests labels used within the last 30 days (`recent_labels`), while
  labels already on the card stay togglable. Keeps clutter down over time.

## v6 (shipped)

- **Column (status) management**, core + TUI + MCP. core gains `add_column`,
  `rename_column`, `reorder_column` and `delete_column(victim, dest)` — delete
  relocates *all* cards (including archived) to a chosen destination column so
  the cascade loses nothing, and refuses to remove the last column.
- **TUI**: a column manager modal (`c`) to add / rename / reorder (`J/K`) /
  delete; deleting prompts for the destination column.
- **MCP**: `manage_columns` (add/rename/delete/reorder) and `manage_boards`
  (create/delete, Backlog board protected) tools — board/column structure is
  shared state agents legitimately need to adjust, so the surface grew from 5 to
  7 deliberately. Hint colours were also brightened for dark terminals
  (HELP/HINT palette).

## v7 (shipped)

- **Board archiving** (migration 0003: `boards.archived_at`). Finished projects
  are archived — hidden from `list_boards`, the TUI tabs and the MCP board
  directory — instead of deleted, so card history survives for cross-session
  recall. `list_boards_all` exposes everything; `archive_board` /
  `unarchive_board` flip the flag.
- **Two-step delete**: `delete_board` now refuses non-archived boards, so the
  destructive cascade always requires archive-then-delete. The Backlog board can
  be neither archived nor deleted.
- **TUI**: `D` archives the current board (y/n confirm), `U` opens an archived
  boards picker — `Enter` unarchives and switches to it, `d` hard-deletes via
  the typed `delete` confirmation.
- **MCP**: `manage_boards` gains `archive` / `unarchive`; `get_board` lists
  archived boards on a separate trailing line so they stay discoverable (for
  unarchive) without cluttering the active directory.

## v8 (shipped)

- **Memory log** (migration 0004: `memories` + `counters`): the other half of
  cross-session continuity. Tasks persist on boards; *why* decisions were made
  now persists as memories — `record_memory(title, body, kind, card_key)` with
  global `M-N` keys allocated from a `counters` row under BEGIN IMMEDIATE (same
  scheme as card keys). `recall_memories` does escaped-LIKE substring search
  over title/body/card_key plus exact card/kind filters, newest first;
  `update_memory` patches/archives. Memories reference cards by key *text*
  (no FK) deliberately: they must outlive board archive/delete.
- **MCP**: `record_memory` / `recall_memories` (key mode returns one memory in
  full). Server instructions now tell agents to recall on topic start and
  record non-obvious decisions. Surface: 9 tools.
- **TUI**: `m` opens a read-only memory browser (newest first, kind/date/card
  chips), `Enter` for full detail, `d` archives (y/n). Recording stays
  agent-side by design.

## v9 (shipped)

- **Memory retention** (migration 0005): memories now track MCP recall
  freshness with `last_recalled_at` and `recall_count`. TUI browsing is
  intentionally read-only for this purpose, so opening the memory browser does
  not keep every memory alive.
- **Monthly GC**: each DB opener performs a cheap counter check and, at most
  once every 30 days, hard-deletes memories older than 30 days that have never
  been recalled. This keeps the memory log from accumulating one-off notes while
  preserving anything an agent has actually used.

## v10 (shipped)

- **Agent workflow fields** (migration 0006): cards now have structured
  `next_action`, `blocked_reason` and `acceptance_criteria` columns. Agents
  update them through the existing `update_card` tool and read them from
  `get_card`; `list_cards(query=...)` also searches them. TUI detail renders
  them read-only so humans can see agent handoff state without adding more edit
  modes.

## v11 (shipped)

- **TUI themes**: `glass` is the default; built-in `dark` / `light` alternatives
  are selected with `KANBAN_THEME`, and `KANBAN_THEME_FILE` can override key
  colors with JSON.
  Priority remains badge-first (`[H]`, `[M]`, `[L]`) so colour is useful but not
  required for understanding.

## v12 (shipped)

- **Agent leases** (migration 0007): cards now carry `claimed_by`,
  `claimed_at` and `lease_expires_at`. Agents claim through the existing
  `update_card` tool with `claim` / `lease_minutes`, release with
  `release_claim`, and a different agent can take over only after the lease
  expires. This is advisory collision avoidance for agents; TUI editing remains
  possible.

## v13 (shipped)

- **Board ordering** (migration 0008): boards now carry `sort_order`. The TUI
  board list (`b`) supports `J/K` reordering, and MCP exposes the same operation
  through `manage_boards(action="reorder", direction="up|down")`.

## v14 (shipped)

- **Planning lanes** (migration 0009): The default board is now displayed as
  Backlog. Its lanes are `Backlog / Today / This week / This month`; older
  Japanese default lanes are renamed, and an empty auto-created `backlog` board
  from the short-lived earlier design is removed. If it contains cards, they are
  moved to the then-default board's `Backlog` lane when there is no key
  collision.
- **Default board display name** (migration 0011): existing databases with a
  `main` board named `Main` are renamed to `Backlog`. Custom names are left
  untouched.
- **Protected Backlog slug** (migration 0012): the default board slug is now
  `backlog`. If a separate `backlog` board already exists, any leftover `main`
  board is dropped; otherwise the old `main` board is renamed in place.
- **Unique Backlog board** (migration 0013): `Backlog` is now a reserved board
  name. Duplicate Backlog boards are dropped, and the protected `backlog` board
  is normalized to exactly one `Backlog` column. Project boards keep the four
  planning columns.

## v15 (shipped)

- **Agent execution metadata** (migration 0016): cards carry `agent_weight`
  (`1..5`), `agent_effort`, `suggested_model`, `expected_tokens` and
  `human_intervention` (`none`, `review`, `decision`, `execution`). This is
  separate from `priority`: priority is human/business importance, while these
  fields describe suitability, runtime cost and human gates for agents.
- **MCP execution surface**: `update_card` reads/writes the metadata, `get_card`
  renders it under `agent_metadata`, and `list_cards` can filter by weight,
  effort, model, token bounds and human intervention. `get_board` stays compact,
  showing only small weight/human hints.
- **Plan ingestion**: `create_cards` turns an ordered spec/plan into durable
  cards with body, column, acceptance criteria, next action and execution
  metadata. Input-local `alias` plus `depends_on` records DAGs after keys are
  allocated.
- **Executable queue**: `list_cards(queue=...)` separates `executable`,
  `review`, `blocked`, `claimed`, `missing_context`, `dependency_blocked` and
  `human` views. `queue=executable` requires an open, unblocked, unclaimed card
  with `next_action`, `acceptance_criteria` and no human execution gate.

The target agent workflow is: spec/plan -> durable DAG cards -> dependency graph
inspection -> queue selection -> claim -> execute -> verify -> complete/memory.
Dependency-aware plans support shapes such as `A -> B/C/D in parallel -> E`
after fan-in.

## v16 (shipped)

- **Undo**: reversible card updates store a full pre-update card snapshot in the
  activity payload. `u` restores the latest eligible update on the active board;
  permanent deletes remain intentionally non-undoable.
- **Optimistic concurrency**: TUI edit modes capture `updated_at` when opened and
  pass it back as `expected_updated_at`, matching the MCP update contract. A
  concurrent external update is rejected instead of being silently overwritten.
- **Wide-character editing**: the body editor keeps its cursor as a character
  index for safe string mutation, then converts the prefix to terminal display
  width when positioning the caret. CJK and mixed-width text therefore align.
- **Durable agent handoffs** (migration 0019): handoffs are a separate leased
  inbox queue addressed to an exact registered identity or an agent family.
  `send_handoff`, `list_handoffs`, `claim_handoff` and `complete_handoff` expose
  the queue without overloading card workflow fields.
- **Runtime delivery**: `watch-handoffs` claims and delivers inbox items through
  command targets or thin bridge scripts. Target YAML keeps routing reusable;
  interactive tmux/zellij target shapes are parsed but remain deferred.
- **Small workflow runner**: workflow YAML supports named steps and an
  `on_complete.send_handoff` action. Card completion can trigger the same runner,
  while `run-agent-task` claims an incoming handoff, executes a command target,
  completes its card, and optionally triggers the next step.
- **Runtime hooks**: the Claude Code hook installer manages only Kanterm-owned
  `SessionStart`, `SessionEnd`, and `Stop` entries and preserves unrelated hooks.

## v17 (shipped)

- **Execution dashboard**: the TUI starts in a primary, active-board view.
  `Tab` / `Shift+Tab` cycle the Kanban, LIST, TIMELINE and FLOW tabs (`1` / `2`
  / `3` / `4` select directly). It groups work into running,
  human-gated, ready, explicitly blocked, dependency-waiting and missing-context
  buckets. Separate title and why/next columns keep dependency keys, blocker
  reasons and next actions visible at normal terminal widths.
- **One execution policy**: dashboard grouping calls `classify_work` from
  `kanterm-core`, the same source used by local next-work navigation and MCP
  queue filtering. The TUI does not duplicate queue precedence.
- **Control-plane navigation**: `j` / `k` moves through the ranked work list and
  `Enter` opens card detail as an overlay on the current execution tab. External
  MCP writes repaint the dashboard through the existing `data_version` refresh.
- **Execution projections**: every execution view is scoped to the active
  board. TIMELINE maps that board's
  `dependency_stage_plan` to a Gantt-like stage axis, keeping parallel cards in
  one column without inventing calendar duration; FLOW places its live
  `classify_work` counts on a state-machine-style map and exposes cards in the
  selected state. Neither view owns execution policy; both are read projections
  over `kanterm-core`.

## Known follow-ups

- Replace the current optimistic-conflict error with a richer TUI recovery
  prompt that can reload the external version or reopen the local edit.
- Implement the reserved interactive target adapters for tmux/zellij without
  moving runtime-specific delivery state into `kanterm-core`.
- Persist workflow-run and per-step state before adding retries, timeouts or
  cancellation; the current YAML runner deliberately stays a small handoff
  trigger rather than a general CI engine.
- Re-verify `rmcp` (pinned `=1.7.0`) on each bump.
