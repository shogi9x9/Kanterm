use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use crate::PROTECTED_BOARD_SLUG;

pub(super) fn ensure_project_board_columns_mutable(
    conn: &rusqlite::Connection,
    board_id: &str,
) -> Result<()> {
    let slug: String = conn
        .query_row(
            "SELECT slug FROM boards WHERE id = ?1",
            params![board_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no such board"))?;
    ensure_mutable_slug(&slug)
}

pub(super) fn ensure_column_board_mutable(
    conn: &rusqlite::Connection,
    column_id: &str,
) -> Result<()> {
    let slug: String = conn
        .query_row(
            "SELECT b.slug
               FROM columns c
               JOIN boards b ON b.id = c.board_id
              WHERE c.id = ?1",
            params![column_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no such column"))?;
    ensure_mutable_slug(&slug)
}

fn ensure_mutable_slug(slug: &str) -> Result<()> {
    if slug == PROTECTED_BOARD_SLUG {
        return Err(anyhow!(
            "the Backlog board must keep exactly one Backlog column"
        ));
    }
    Ok(())
}
