use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::rows::{row_to_card, CARD_SELECT_BY_ID};
use crate::Card;

mod claim;
mod create;
mod fields;
mod read;
mod relocate;
mod snapshot;
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

#[cfg(test)]
mod tests {
    use crate::{CardPatch, Store};

    #[test]
    fn blank_claim_releases_existing_claim() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "claim me", "", "test")
            .unwrap();
        let agent = store.register_agent("agent-a", None, None, None).unwrap();

        store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    claim: Some(agent.registration.assigned_identity.clone()),
                    claim_token: Some(agent.claim_token.clone()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();
        let released = store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    claim: Some(" ".to_string()),
                    claim_token: Some(agent.claim_token),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        assert_eq!(released.claimed_by, None);
        assert_eq!(released.claimed_at, None);
        assert_eq!(released.lease_expires_at, None);
    }

    #[test]
    fn blank_workflow_fields_clear_optional_values() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "workflow", "", "test")
            .unwrap();

        store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    next_action: Some("Run tests".to_string()),
                    blocked_reason: Some("Waiting".to_string()),
                    acceptance_criteria: Some("All green".to_string()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();
        let cleared = store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    next_action: Some(" ".to_string()),
                    blocked_reason: Some("".to_string()),
                    acceptance_criteria: Some(" ".to_string()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        assert_eq!(cleared.next_action, None);
        assert_eq!(cleared.blocked_reason, None);
        assert_eq!(cleared.acceptance_criteria, None);
    }
}
