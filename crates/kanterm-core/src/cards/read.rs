use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::rows::{
    row_to_card, CARD_SELECT_BY_ID, CARD_SELECT_BY_KEY, CARD_SELECT_PREFIX_WHERE_BOARD,
};
use crate::{Card, Store};

impl Store {
    /// All non-archived cards on a board, ordered by column then position.
    pub fn cards(&self, board_id: &str) -> Result<Vec<Card>> {
        let mut stmt = self.conn.prepare(CARD_SELECT_PREFIX_WHERE_BOARD)?;
        let rows = stmt
            .query_map(params![board_id], row_to_card)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn card_by_key(&self, board_id: &str, key: &str) -> Result<Option<Card>> {
        self.conn
            .query_row(CARD_SELECT_BY_KEY, params![board_id, key], row_to_card)
            .optional()
            .map_err(Into::into)
    }

    pub fn card_by_id(&self, card_id: &str) -> Result<Option<Card>> {
        self.conn
            .query_row(CARD_SELECT_BY_ID, params![card_id], row_to_card)
            .optional()
            .map_err(Into::into)
    }
}
