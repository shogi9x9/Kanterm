use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::rows::{row_to_card, CARD_SELECT_BY_ID};
use crate::Card;

mod claim;
mod create;
mod fields;
mod plan;
mod read;
mod relocate;
mod snapshot;
#[cfg(test)]
mod tests;
mod update;

/// Load a single card by its internal id inside an open transaction. Shared by
/// the create and update flows, which need to return the freshly written row.
pub(super) fn load_card_by_id_tx(tx: &Transaction<'_>, card_id: &str) -> Result<Card> {
    tx.query_row(CARD_SELECT_BY_ID, params![card_id], row_to_card)
        .optional()?
        .ok_or_else(|| anyhow!("no card id '{card_id}'"))
}

/// Bump a card's `updated_at`. Shared by create and update flows.
pub(super) fn touch_card(tx: &Transaction<'_>, card_id: &str, ts: i64) -> Result<()> {
    tx.execute(
        "UPDATE cards SET updated_at = ?1 WHERE id = ?2",
        params![ts, card_id],
    )?;
    Ok(())
}
