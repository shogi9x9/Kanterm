use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::id::new_id;
use crate::position::next_position;
use crate::rows::row_to_column;
use crate::{now_ms, Column, Store, PROTECTED_BOARD_SLUG};

impl Store {
    pub fn columns(&self, board_id: &str) -> Result<Vec<Column>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, board_id, name, sort_order, wip_limit FROM columns
             WHERE board_id = ?1 ORDER BY sort_order",
        )?;
        let rows = stmt
            .query_map(params![board_id], row_to_column)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Append a new column to the end of a board. Names are unique per board.
    pub fn add_column(&mut self, board_id: &str, name: &str) -> Result<Column> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("column name must not be empty"));
        }
        self.assert_writable()?;
        self.ensure_project_board_columns_mutable(board_id)?;
        let max: Option<i64> = self.conn.query_row(
            "SELECT MAX(sort_order) FROM columns WHERE board_id = ?1",
            params![board_id],
            |r| r.get(0),
        )?;
        let order = max.map(|m| m + 1).unwrap_or(0);
        let id = new_id();
        self.conn
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

    /// Rename a column. Fails if the new name collides on the same board.
    pub fn rename_column(&mut self, column_id: &str, new_name: &str) -> Result<()> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(anyhow!("column name must not be empty"));
        }
        self.assert_writable()?;
        self.ensure_column_board_mutable(column_id)?;
        self.conn
            .execute(
                "UPDATE columns SET name = ?1 WHERE id = ?2",
                params![new_name, column_id],
            )
            .map_err(|e| anyhow!("could not rename column: {e}"))?;
        Ok(())
    }

    /// Move a column left (-1) or right (+1) by swapping sort_order with its
    /// neighbour. No-op at the ends.
    pub fn reorder_column(&mut self, board_id: &str, column_id: &str, dir: i32) -> Result<()> {
        if dir == 0 {
            return Ok(());
        }
        self.assert_writable()?;
        self.ensure_project_board_columns_mutable(board_id)?;
        let tx = self
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

    /// Delete a column, relocating its cards (including archived ones) to
    /// `dest_id`. Refuses to delete the last column or move into itself.
    pub fn delete_column(&mut self, board_id: &str, victim_id: &str, dest_id: &str) -> Result<()> {
        if victim_id == dest_id {
            return Err(anyhow!("destination column must be different"));
        }
        self.assert_writable()?;
        self.ensure_project_board_columns_mutable(board_id)?;
        let tx = self
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
        let in_board = |tx: &rusqlite::Transaction, id: &str| -> Result<bool> {
            Ok(tx
                .query_row(
                    "SELECT 1 FROM columns WHERE id = ?1 AND board_id = ?2",
                    params![id, board_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some())
        };
        if !in_board(&tx, victim_id)? || !in_board(&tx, dest_id)? {
            return Err(anyhow!("column not found on this board"));
        }

        let movers: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT id FROM cards
                 WHERE column_id = ?1 AND archived_at IS NULL ORDER BY position",
            )?;
            let rows = stmt
                .query_map(params![victim_id], |r| r.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
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

    fn ensure_project_board_columns_mutable(&self, board_id: &str) -> Result<()> {
        let slug: String = self
            .conn
            .query_row(
                "SELECT slug FROM boards WHERE id = ?1",
                params![board_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no such board"))?;
        if slug == PROTECTED_BOARD_SLUG {
            return Err(anyhow!(
                "the Backlog board must keep exactly one Backlog column"
            ));
        }
        Ok(())
    }

    fn ensure_column_board_mutable(&self, column_id: &str) -> Result<()> {
        let slug: String = self
            .conn
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
        if slug == PROTECTED_BOARD_SLUG {
            return Err(anyhow!(
                "the Backlog board must keep exactly one Backlog column"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_column_rejects_blank_name() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();

        let err = store.add_column(&board.id, "   ").unwrap_err().to_string();

        assert!(err.contains("column name must not be empty"));
    }

    #[test]
    fn backlog_board_columns_are_not_mutable() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let column = store.columns(&board.id).unwrap().remove(0);

        let add = store
            .add_column(&board.id, "Today")
            .unwrap_err()
            .to_string();
        assert!(add.contains("must keep exactly one Backlog column"));

        let rename = store
            .rename_column(&column.id, "Inbox")
            .unwrap_err()
            .to_string();
        assert!(rename.contains("must keep exactly one Backlog column"));
    }
}
