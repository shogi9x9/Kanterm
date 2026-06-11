# Move cards across boards via MCP

## Overview

`update_card` supports `move_to_board`, which moves a card from its current board to another board.
The default `backlog` board is a reserved inbox/planning board with exactly one
`Backlog` column; project boards are the usual destination once work is ready to
schedule.

- `key` is the source card key (for example, `KB-1`).
- `move_to_board` is the destination board **slug** (for example, `work`).
- Optional `column` moves the card directly into a named column on the destination board.

## Example

```json
{
  "tool": "update_card",
  "arguments": {
    "key": "KB-1",
    "move_to_board": "work",
    "column": "This week"
  }
}
```

## Behavioral notes

- Moving a card changes its board context, so the key is reissued using the destination
  board's key prefix (for example: `KB-1` -> `WB-1`).
- If `column` is omitted, the card is moved to the first column of the destination board.
- The Backlog board only has a `Backlog` column. Project-board columns depend on
  the template used at board creation; use `get_board` to check the destination
  columns before passing `column`.
- Use `get_board` first to check available board slugs.

## Common error cases

- Unknown `move_to_board` slug: returns an error.
- Unknown `column` name on the destination board: returns an error.

### Related MCP operations

- Inspect boards: `get_board` / `list_cards`
- Create destination project board if needed: `manage_boards` with `action: create`
- Cross-board move: call `update_card` with `move_to_board`
