use anyhow::Result;
use rusqlite::params;

use crate::rows::row_to_column;
use crate::Column;

pub(super) fn columns(conn: &rusqlite::Connection, board_id: &str) -> Result<Vec<Column>> {
    let mut stmt = conn.prepare(
        "SELECT id, board_id, name, sort_order, wip_limit FROM columns
         WHERE board_id = ?1 ORDER BY sort_order",
    )?;
    let rows = stmt
        .query_map(params![board_id], row_to_column)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
