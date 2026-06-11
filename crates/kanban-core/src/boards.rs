use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::id::new_id;
use crate::naming::{derive_prefix, derive_slug};
use crate::rows::{row_to_board, BOARD_COLUMNS};
use crate::{
    now_ms, Board, BoardColumnTemplate, Store, BACKLOG_BOARD_COLUMNS, PROTECTED_BOARD_SLUG,
};

/// Insert a board and its default columns. Returns the board id.
pub(crate) fn insert_board(
    tx: &rusqlite::Transaction,
    name: &str,
    slug: &str,
    prefix: &str,
    columns: &[&str],
) -> Result<String> {
    let board_id = new_id();
    let ts = now_ms();
    let sort_order = tx.query_row(
        "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM boards",
        [],
        |r| r.get::<_, i64>(0),
    )?;
    tx.execute(
        "INSERT INTO boards (id, name, slug, key_prefix, card_seq, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)",
        params![board_id, name, slug, prefix, sort_order, ts],
    )?;
    for (order, col) in columns.iter().enumerate() {
        tx.execute(
            "INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            params![new_id(), board_id, col, order as i64, ts],
        )?;
    }
    Ok(board_id)
}

impl Store {
    /// Return the default board, creating it if absent.
    /// This is the board the TUI/MCP fall back to when none is specified.
    pub fn ensure_default_board(&mut self) -> Result<Board> {
        self.ensure_system_board("Backlog", PROTECTED_BOARD_SLUG, "KB", BACKLOG_BOARD_COLUMNS)
    }

    fn ensure_system_board(
        &mut self,
        name: &str,
        slug: &str,
        prefix: &str,
        columns: &[&str],
    ) -> Result<Board> {
        if let Some(b) = self.board_by_slug(slug)? {
            return Ok(b);
        }
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Another process may have created the board between our check and here;
        // on the unique-constraint loss, roll back and re-read its board.
        if insert_board(&tx, name, slug, prefix, columns).is_ok() {
            tx.commit()?;
        } else {
            drop(tx);
        }
        self.board_by_slug(slug)?
            .ok_or_else(|| anyhow!("board creation failed"))
    }

    /// Create a new board from a display name, deriving a unique slug and a key
    /// prefix automatically. Columns are selected by the requested template.
    pub fn create_board(&mut self, name: &str, template: BoardColumnTemplate) -> Result<Board> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("board name must not be empty"));
        }
        self.assert_writable()?;
        let prefix = derive_prefix(name);
        let base = derive_slug(name);
        if base == PROTECTED_BOARD_SLUG {
            return Err(anyhow!("Backlog is the reserved default board"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut slug = base.clone();
        let mut n = 2;
        while tx
            .query_row(
                "SELECT 1 FROM boards WHERE slug = ?1",
                params![slug],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            slug = format!("{base}-{n}");
            n += 1;
        }
        insert_board(&tx, name, &slug, &prefix, template.columns())?;
        tx.commit()?;
        self.board_by_slug(&slug)?
            .ok_or_else(|| anyhow!("board creation failed"))
    }

    /// Active (non-archived) boards. This is what board pickers should show.
    pub fn list_boards(&self) -> Result<Vec<Board>> {
        let sql = format!(
            "SELECT {BOARD_COLUMNS} FROM boards
             WHERE archived_at IS NULL ORDER BY sort_order, created_at"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], row_to_board)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Every board, archived ones included.
    pub fn list_boards_all(&self) -> Result<Vec<Board>> {
        let sql = format!(
            "SELECT {BOARD_COLUMNS} FROM boards
             ORDER BY archived_at IS NOT NULL, sort_order, created_at"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], row_to_board)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Archive a board: it disappears from `list_boards` but keeps all its
    /// columns/cards. The Backlog board cannot be archived.
    pub fn archive_board(&mut self, board_id: &str) -> Result<()> {
        self.assert_writable()?;
        let board = self.board_by_id(board_id)?;
        if board.slug == PROTECTED_BOARD_SLUG {
            return Err(anyhow!("the Backlog board cannot be archived"));
        }
        self.conn.execute(
            "UPDATE boards SET archived_at = ?1 WHERE id = ?2 AND archived_at IS NULL",
            params![now_ms(), board_id],
        )?;
        Ok(())
    }

    pub fn unarchive_board(&mut self, board_id: &str) -> Result<()> {
        self.assert_writable()?;
        self.conn.execute(
            "UPDATE boards SET archived_at = NULL WHERE id = ?1",
            params![board_id],
        )?;
        Ok(())
    }

    /// Move an active board earlier (-1) or later (+1) by swapping sort_order
    /// with its active neighbour. No-op at the ends.
    pub fn reorder_board(&mut self, board_id: &str, dir: i32) -> Result<()> {
        if dir == 0 {
            return Ok(());
        }
        self.assert_writable()?;
        let tx = self
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

    /// Delete a board and everything on it (columns/cards cascade).
    /// Only archived boards can be deleted.
    pub fn delete_board(&mut self, board_id: &str) -> Result<()> {
        self.assert_writable()?;
        let board = self.board_by_id(board_id)?;
        if board.slug == PROTECTED_BOARD_SLUG {
            return Err(anyhow!("the Backlog board cannot be deleted"));
        }
        if board.archived_at.is_none() {
            return Err(anyhow!(
                "board '{}' is not archived; archive it first, then delete",
                board.slug
            ));
        }
        self.conn
            .execute("DELETE FROM boards WHERE id = ?1", params![board_id])?;
        Ok(())
    }

    /// Set or clear board-level agent execution guidance. Empty/whitespace text clears it.
    pub fn update_board_agent_context(
        &mut self,
        board_id: &str,
        agent_context: Option<&str>,
    ) -> Result<Board> {
        self.assert_writable()?;
        let normalized = agent_context.and_then(|text| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        self.conn.execute(
            "UPDATE boards SET agent_context = ?1, updated_at = ?2 WHERE id = ?3",
            params![normalized, now_ms(), board_id],
        )?;
        self.board_by_id(board_id)
    }

    fn board_by_id(&self, board_id: &str) -> Result<Board> {
        self.conn
            .query_row(
                &format!("SELECT {BOARD_COLUMNS} FROM boards WHERE id = ?1"),
                params![board_id],
                row_to_board,
            )
            .optional()?
            .ok_or_else(|| anyhow!("no such board"))
    }

    pub fn board_by_slug(&self, slug: &str) -> Result<Option<Board>> {
        self.conn
            .query_row(
                &format!("SELECT {BOARD_COLUMNS} FROM boards WHERE slug = ?1"),
                params![slug],
                row_to_board,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn board_by_id_or_slug(&self, value: &str) -> Result<Board> {
        self.board_by_slug(value)?
            .map(Ok)
            .unwrap_or_else(|| self.board_by_id(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_archive_backlog_board() {
        let mut store = Store::open_in_memory().unwrap();
        let backlog = store.ensure_default_board().unwrap();

        let err = store.archive_board(&backlog.id).unwrap_err().to_string();

        assert!(err.contains("Backlog board cannot be archived"));
    }

    #[test]
    fn cannot_create_another_backlog_board() {
        let mut store = Store::open_in_memory().unwrap();
        store.ensure_default_board().unwrap();

        let err = store
            .create_board("  BACKLOG  ", BoardColumnTemplate::Planning)
            .unwrap_err()
            .to_string();

        assert!(err.contains("Backlog is the reserved default board"));
        assert!(store.board_by_slug("backlog-2").unwrap().is_none());
    }

    #[test]
    fn board_reorder_swaps_active_neighbours() {
        let mut store = Store::open_in_memory().unwrap();
        let first = store.ensure_default_board().unwrap();
        let second = store
            .create_board("Second Board", BoardColumnTemplate::Planning)
            .unwrap();

        store.reorder_board(&second.id, -1).unwrap();

        let boards = store.list_boards().unwrap();
        assert_eq!(boards[0].id, second.id);
        assert_eq!(boards[1].id, first.id);
    }

    #[test]
    fn create_board_uses_selected_column_template() {
        let mut store = Store::open_in_memory().unwrap();
        store.ensure_default_board().unwrap();

        let workflow = store
            .create_board("Release Work", BoardColumnTemplate::Workflow)
            .unwrap();
        let columns: Vec<String> = store
            .columns(&workflow.id)
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect();

        assert_eq!(
            columns,
            BoardColumnTemplate::Workflow
                .columns()
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn default_project_template_is_workflow() {
        assert_eq!(
            BoardColumnTemplate::DEFAULT_PROJECT,
            BoardColumnTemplate::Workflow
        );
        assert_eq!(
            BoardColumnTemplate::ALL[BoardColumnTemplate::default_index()],
            BoardColumnTemplate::Workflow
        );
    }

    #[test]
    fn board_agent_context_trims_and_clears() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();

        let updated = store
            .update_board_agent_context(&board.id, Some("  Run cargo test before closing.  "))
            .unwrap();
        assert_eq!(
            updated.agent_context.as_deref(),
            Some("Run cargo test before closing.")
        );

        let cleared = store
            .update_board_agent_context(&board.id, Some("  "))
            .unwrap();
        assert_eq!(cleared.agent_context, None);
    }
}
