use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::position::next_position;
use crate::{now_ms, Store};

use super::guard;

pub(super) fn delete_column(
    store: &mut Store,
    board_id: &str,
    victim_id: &str,
    dest_id: &str,
) -> Result<()> {
    if victim_id == dest_id {
        return Err(anyhow!("destination column must be different"));
    }
    store.assert_writable()?;
    guard::ensure_project_board_columns_mutable(&store.conn, board_id)?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;

    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM columns WHERE board_id = ?1",
        params![board_id],
        |r| r.get(0),
    )?;
    if count <= 1 {
        return Err(anyhow!("cannot delete the only column"));
    }
    if !in_board(&tx, board_id, victim_id)? || !in_board(&tx, board_id, dest_id)? {
        return Err(anyhow!("column not found on this board"));
    }

    let movers: Vec<String> = {
        let mut stmt = tx.prepare(
            "SELECT id FROM cards
             WHERE column_id = ?1 AND archived_at IS NULL ORDER BY position",
        )?;
        let rows = stmt.query_map(params![victim_id], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut next = next_position(&tx, dest_id)?;
    let ts = now_ms();
    for cid in movers {
        tx.execute(
            "UPDATE cards SET column_id = ?1, position = ?2, updated_at = ?3 WHERE id = ?4",
            params![dest_id, next, ts, cid],
        )?;
        next += 1.0;
    }
    tx.execute(
        "UPDATE cards SET column_id = ?1 WHERE column_id = ?2",
        params![dest_id, victim_id],
    )?;
    tx.execute("DELETE FROM columns WHERE id = ?1", params![victim_id])?;
    tx.commit()?;
    Ok(())
}

fn in_board(tx: &rusqlite::Transaction<'_>, board_id: &str, id: &str) -> Result<bool> {
    Ok(tx
        .query_row(
            "SELECT 1 FROM columns WHERE id = ?1 AND board_id = ?2",
            params![id, board_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
