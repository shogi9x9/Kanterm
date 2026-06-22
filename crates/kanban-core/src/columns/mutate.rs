use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::id::new_id;
use crate::{now_ms, Column, Store};

use super::guard;

pub(super) fn add_column(store: &mut Store, board_id: &str, name: &str) -> Result<Column> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("column name must not be empty"));
    }
    store.assert_writable()?;
    guard::ensure_project_board_columns_mutable(&store.conn, board_id)?;
    let max: Option<i64> = store.conn.query_row(
        "SELECT MAX(sort_order) FROM columns WHERE board_id = ?1",
        params![board_id],
        |r| r.get(0),
    )?;
    let order = max.map(|m| m + 1).unwrap_or(0);
    let id = new_id();
    store
        .conn
        .execute(
            "INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            params![id, board_id, name, order, now_ms()],
        )
        .map_err(|e| anyhow!("could not add column '{name}': {e}"))?;
    Ok(Column {
        id,
        board_id: board_id.to_string(),
        name: name.to_string(),
        sort_order: order,
        wip_limit: None,
    })
}

pub(super) fn rename_column(store: &mut Store, column_id: &str, new_name: &str) -> Result<()> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(anyhow!("column name must not be empty"));
    }
    store.assert_writable()?;
    guard::ensure_column_board_mutable(&store.conn, column_id)?;
    store
        .conn
        .execute(
            "UPDATE columns SET name = ?1 WHERE id = ?2",
            params![new_name, column_id],
        )
        .map_err(|e| anyhow!("could not rename column: {e}"))?;
    Ok(())
}

pub(super) fn reorder_column(
    store: &mut Store,
    board_id: &str,
    column_id: &str,
    dir: i32,
) -> Result<()> {
    if dir == 0 {
        return Ok(());
    }
    store.assert_writable()?;
    guard::ensure_project_board_columns_mutable(&store.conn, board_id)?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let order: i64 = tx
        .query_row(
            "SELECT sort_order FROM columns WHERE id = ?1 AND board_id = ?2",
            params![column_id, board_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no such column"))?;
    let neighbour: Option<(String, i64)> = if dir < 0 {
        tx.query_row(
            "SELECT id, sort_order FROM columns
             WHERE board_id = ?1 AND sort_order < ?2 ORDER BY sort_order DESC LIMIT 1",
            params![board_id, order],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    } else {
        tx.query_row(
            "SELECT id, sort_order FROM columns
             WHERE board_id = ?1 AND sort_order > ?2 ORDER BY sort_order ASC LIMIT 1",
            params![board_id, order],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    };
    if let Some((other_id, other_order)) = neighbour {
        tx.execute(
            "UPDATE columns SET sort_order = ?1 WHERE id = ?2",
            params![other_order, column_id],
        )?;
        tx.execute(
            "UPDATE columns SET sort_order = ?1 WHERE id = ?2",
            params![order, other_id],
        )?;
        tx.commit()?;
    }
    Ok(())
}
