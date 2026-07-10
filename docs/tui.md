# TUI reference

日本語版: [tui.ja.md](tui.ja.md)

```sh
./target/release/kanterm
```

## Board keys

| Key            | Action                              |
| -------------- | ----------------------------------- |
| `h` / `l`      | move focus between columns          |
| `j` / `k`      | move selection within a column      |
| `H` / `L`      | move the selected card left/right   |
| `J` / `K`      | reorder the card down/up in column  |
| `n`            | new card (in focused column)        |
| `N`            | new board                           |
| `b`            | select board from a list (`J/K` reorders boards) |
| `i`            | edit the current board's agent context (empty clears) |
| `M`            | move selected card to another board/column (`send-project` on Backlog) |
| `c`            | manage columns (add/rename/reorder/delete) |
| `Tab`          | switch to the next board            |
| `w`            | jump to the next local work candidate |
| `/`            | filter cards (title/body/label)     |
| `Enter`        | open the card **detail modal**      |
| `e`            | quick-edit the selected card's title |
| `p`            | cycle priority (`[L]` → `[M]` → `[H]`) |
| `u`            | undo the latest reversible card update |
| `d`            | archive selected card               |
| `m`            | memory browser (decisions/learnings) |
| `D`            | archive the current board           |
| `U`            | archived boards: `Enter` unarchive, `d` delete |
| `r`            | reload from disk                    |
| `q` / `Esc`    | quit                                |

`/` opens a filter that narrows every column to cards matching the text in their
title, body or labels; submit an empty filter to clear it.

## Detail modal

In the **detail modal**: `e` title · `b` edit body (multi-line) · `M` move to
another board/column · `p` priority · `a` assignee · `D` due date (`YYYY-MM-DD`,
empty to clear) · `t` labels · `x` complete with an optional note · `d` archive ·
`Esc` back. Completing a card archives it, marks `agent_state=done`, clears active
handoff fields, and releases any claim. Overdue cards show a red `⏰` chip.

From the board, `u` restores the latest reversible card update on the active
board, including accidental archive, complete and card move operations. Hard
deletes remain intentionally non-undoable.

In the **body editor**: arrows move, `Enter` newline, `Ctrl-S` save, `Esc` cancel.

## Label picker

The **label picker** (`t`) is a popup: type a name and `Enter` to add a new
label, or use `↑/↓` and `Space` to toggle previously-used labels on/off (attach
as many as you like). To keep the list tidy, only labels used within the last
month are suggested — labels still on the card always remain togglable.

## Boards

The default board is named **Backlog** and uses the `backlog` slug. It is a
reserved inbox/planning board with exactly one column: **Backlog**. You cannot
create another board named Backlog, and you cannot add, rename, reorder, or
delete columns on the Backlog board.

Project boards default to the developer-oriented `workflow` template
(**Todo · In progress · Testing · Waiting for release**). You can also choose
`planning` (**Backlog · Today · This week · This month**) or `simple`
(**Todo · Doing · Done**). Manage columns with `c`: add, rename, reorder
(`J/K`), or delete — deleting a column asks which column its cards should move to.

The board remembers your focused column, selected card and **active board**
between launches. A header row shows the active board as a compact selector
(`board < name > 1/N`); use `Tab` to cycle boards or `b` to open the board list.

Finished boards are **archived** (`D`), not deleted: they vanish from the tabs
but keep all their cards. `U` lists archived boards to unarchive (`Enter`) or
permanently delete (`d`, then type `delete`) — deletion requires archiving
first, and the Backlog board can do neither.

## Themes

The TUI uses `glass` by default and also ships with `dark` and `light` themes:

```sh
KANBAN_THEME=light ./target/release/kanterm
```

`glass` leaves selections on the terminal's default background, uses a subtle
selection marker, and adds breathing room between columns. Pair it with your
terminal emulator's background opacity setting for a translucent appearance.
It can also be selected explicitly:

```sh
KANBAN_THEME=glass ./target/release/kanterm
```

Override key colors with a JSON file and `KANBAN_THEME_FILE`:

```json
{
  "accent": "cyan",
  "warning": "yellow",
  "danger": "red",
  "success": "green",
  "priority_high": "#ff5555",
  "priority_normal": "yellow",
  "priority_low": "blue"
}
```

Supported values are ANSI color names such as `red`, `light_cyan`, `dark_gray`,
the terminal background aliases `reset` / `default`, or hex colors like
`#ff5555`.

## Memory log

Alongside the boards lives a **memory log**: decisions, learnings and context
that should survive across agent sessions, addressed by global keys (`M-1`,
`M-2`, …) and optionally linked to a card. Agents write and search it via the
MCP tools (`record_memory` / `recall_memories`); in the TUI, `m` opens a
read-only browser (`Enter` detail, `d` archive). Memories link to cards only by
key text, so they outlive board archiving and deletion. Agent recall is tracked
separately from TUI browsing; once a month, memories older than 30 days with no
agent recalls are purged.

## Export

```sh
./target/release/kanterm --export md     # Markdown (good for git)
./target/release/kanterm --export json   # full JSON snapshot
```

## Backup / restore

Use SQLite-level backups for restoration. `--export` is for reviewable snapshots,
not lossless import.

```sh
./target/release/kanterm --backup-db ./kanban-backup.db
./target/release/kanterm --restore-db ./kanban-backup.db --force
```

Backups are written with SQLite `VACUUM INTO`, so pending WAL contents are
captured consistently. Restore validates the source schema version first and
refuses to replace an existing database unless `--force` is present. A backup
from a newer kanban build is rejected rather than migrated backward.
Before testing a new local release binary against your real board, take a
timestamped backup outside the repo, for example:

```sh
./target/release/kanterm --backup-db ~/kanban-backups/kanban-$(date +%Y%m%d-%H%M%S).db
```
