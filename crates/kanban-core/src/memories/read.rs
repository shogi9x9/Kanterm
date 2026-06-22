use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::rows::{row_to_memory, MEMORY_COLUMNS};
use crate::Memory;

pub(super) fn memory_by_key(conn: &rusqlite::Connection, key: &str) -> Result<Option<Memory>> {
    conn.query_row(
        &format!("SELECT {MEMORY_COLUMNS} FROM memories WHERE key_text = ?1"),
        params![key],
        row_to_memory,
    )
    .optional()
    .map_err(Into::into)
}
