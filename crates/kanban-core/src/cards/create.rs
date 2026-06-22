use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::activity::log_activity;
use crate::id::new_id;
use crate::position::next_position;
use crate::search::sync_card_search_row;
use crate::{now_ms, Card, Store};

impl Store {
    /// Create a card in the named column (defaults to the first column).
    pub fn create_card(
        &mut self,
        board_id: &str,
        column_name: Option<&str>,
        title: &str,
        body: &str,
        actor: &str,
    ) -> Result<Card> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let column_id = resolve_create_column(&tx, board_id, column_name)?;

        let (prefix, seq): (String, i64) = tx.query_row(
            "SELECT key_prefix, card_seq FROM boards WHERE id = ?1",
            params![board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let next_seq = seq + 1;
        let key = format!("{prefix}-{next_seq}");
        let position = next_position(&tx, &column_id)?;
        let ts = now_ms();
        let id = new_id();

        tx.execute(
            "INSERT INTO cards
               (id, board_id, column_id, key_text, title, body, status, priority,
                assignee, due_date, position, created_at, updated_at, archived_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', 1, NULL, NULL, ?7, ?8, ?8, NULL)",
            params![id, board_id, column_id, key, title, body, position, ts],
        )?;
        tx.execute(
            "UPDATE boards SET card_seq = ?1, updated_at = ?2 WHERE id = ?3",
            params![next_seq, ts, board_id],
        )?;
        log_activity(&tx, &id, actor, "create", &key)?;
        sync_card_search_row(&tx, &id)?;
        tx.commit()?;

        self.card_by_key(board_id, &key)?
            .ok_or_else(|| anyhow!("card disappeared after insert"))
    }
}

pub(super) fn resolve_create_column(
    tx: &Transaction<'_>,
    board_id: &str,
    column_name: Option<&str>,
) -> Result<String> {
    match column_name {
        Some(name) => tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 AND name = ?2",
                params![board_id, name],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no column named '{name}'")),
        None => tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 ORDER BY sort_order LIMIT 1",
                params![board_id],
                |r| r.get(0),
            )
            .map_err(Into::into),
    }
}
