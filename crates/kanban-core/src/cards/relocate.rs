use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{Card, CardPatch, Store};

impl Store {
    /// Move a card to the end of an adjacent column by name. Convenience for the
    /// TUI's h/l keys; equally expressible via `update_card` with `column`.
    pub fn move_card(
        &mut self,
        board_id: &str,
        key: &str,
        column_name: &str,
        actor: &str,
    ) -> Result<Card> {
        let patch = CardPatch {
            column: Some(column_name.to_string()),
            ..Default::default()
        };
        self.update_card(board_id, key, &patch, actor)
    }

    /// Move a card up (-1) or down (+1) within its own column by swapping its
    /// fractional position with the adjacent card. No-op at the ends.
    pub fn reorder_card(&mut self, board_id: &str, key: &str, dir: i32) -> Result<()> {
        if dir == 0 {
            return Ok(());
        }
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let (card_id, column_id, position): (String, String, f64) = tx
            .query_row(
                "SELECT id, column_id, position FROM cards
                 WHERE board_id = ?1 AND key_text = ?2 AND archived_at IS NULL",
                params![board_id, key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no card '{key}'"))?;

        let neighbour: Option<(String, f64)> = if dir < 0 {
            tx.query_row(
                "SELECT id, position FROM cards
                 WHERE column_id = ?1 AND archived_at IS NULL AND position < ?2
                 ORDER BY position DESC LIMIT 1",
                params![column_id, position],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
        } else {
            tx.query_row(
                "SELECT id, position FROM cards
                 WHERE column_id = ?1 AND archived_at IS NULL AND position > ?2
                 ORDER BY position ASC LIMIT 1",
                params![column_id, position],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
        };

        if let Some((other_id, other_pos)) = neighbour {
            tx.execute(
                "UPDATE cards SET position = ?1 WHERE id = ?2",
                params![other_pos, card_id],
            )?;
            tx.execute(
                "UPDATE cards SET position = ?1 WHERE id = ?2",
                params![position, other_id],
            )?;
            tx.commit()?;
        }
        Ok(())
    }
}
