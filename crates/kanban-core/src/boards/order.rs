use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{now_ms, Store};

pub(super) fn reorder_board(store: &mut Store, board_id: &str, dir: i32) -> Result<()> {
    if dir == 0 {
        return Ok(());
    }
    store.assert_writable()?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (order, archived_at): (i64, Option<i64>) = tx
        .query_row(
            "SELECT sort_order, archived_at FROM boards WHERE id = ?1",
            params![board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no such board"))?;
    if archived_at.is_some() {
        return Err(anyhow!("archived boards cannot be reordered"));
    }
    let neighbour: Option<(String, i64)> = if dir < 0 {
        tx.query_row(
            "SELECT id, sort_order FROM boards
             WHERE archived_at IS NULL AND sort_order < ?1
             ORDER BY sort_order DESC, created_at DESC LIMIT 1",
            params![order],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    } else {
        tx.query_row(
            "SELECT id, sort_order FROM boards
             WHERE archived_at IS NULL AND sort_order > ?1
             ORDER BY sort_order ASC, created_at ASC LIMIT 1",
            params![order],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    };
    if let Some((other_id, other_order)) = neighbour {
        tx.execute(
            "UPDATE boards SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![other_order, now_ms(), board_id],
        )?;
        tx.execute(
            "UPDATE boards SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![order, now_ms(), other_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}
