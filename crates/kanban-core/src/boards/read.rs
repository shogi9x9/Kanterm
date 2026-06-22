use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use crate::rows::{row_to_board, BOARD_COLUMNS};
use crate::Board;

pub(super) fn list_boards(conn: &rusqlite::Connection) -> Result<Vec<Board>> {
    let sql = format!(
        "SELECT {BOARD_COLUMNS} FROM boards
         WHERE archived_at IS NULL ORDER BY sort_order, created_at"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], row_to_board)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub(super) fn list_boards_all(conn: &rusqlite::Connection) -> Result<Vec<Board>> {
    let sql = format!(
        "SELECT {BOARD_COLUMNS} FROM boards
         ORDER BY archived_at IS NOT NULL, sort_order, created_at"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], row_to_board)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub(super) fn board_by_id(conn: &rusqlite::Connection, board_id: &str) -> Result<Board> {
    conn.query_row(
        &format!("SELECT {BOARD_COLUMNS} FROM boards WHERE id = ?1"),
        params![board_id],
        row_to_board,
    )
    .optional()?
    .ok_or_else(|| anyhow!("no such board"))
}

pub(super) fn board_by_slug(conn: &rusqlite::Connection, slug: &str) -> Result<Option<Board>> {
    conn.query_row(
        &format!("SELECT {BOARD_COLUMNS} FROM boards WHERE slug = ?1"),
        params![slug],
        row_to_board,
    )
    .optional()
    .map_err(Into::into)
}
