use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::activity::log_activity_payload;
use crate::search::sync_card_search_row;
use crate::{now_ms, Card, CardPatch, Store};

use super::claim::apply_claim_patch;
use super::fields::{
    apply_board_rehome, apply_column_move, apply_due_date, apply_execution_metadata,
    apply_label_changes, apply_scalar_fields, apply_workflow_fields,
};
use super::{load_card_by_id_tx, touch_card};

impl Store {
    /// Apply a partial update to a card identified by its key.
    pub fn update_card(
        &mut self,
        board_id: &str,
        key: &str,
        patch: &CardPatch,
        actor: &str,
    ) -> Result<Card> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let state = load_card_update_state(&tx, board_id, key)?;
        let undo = load_card_by_id_tx(&tx, &state.id)?;
        let ts = checked_update_timestamp(&state, patch, key)?;

        apply_scalar_fields(&tx, &state.id, patch, ts)?;
        let mut target_board_id = board_id.to_string();
        let board_move =
            apply_board_rehome(&tx, board_id, &mut target_board_id, &state.id, key, patch)?;
        apply_column_move(&tx, &target_board_id, &state.id, patch)?;
        apply_label_changes(&tx, &state.id, patch, ts)?;
        apply_due_date(&tx, &state.id, patch)?;
        apply_workflow_fields(&tx, &state.id, patch)?;
        apply_execution_metadata(&tx, &state.id, patch)?;
        apply_claim_patch(&tx, &state, patch, key, ts)?;
        touch_card(&tx, &state.id, ts)?;
        sync_card_search_row(&tx, &state.id)?;
        if let Some(board_move) = board_move {
            log_activity_payload(
                &tx,
                &state.id,
                actor,
                "move_board",
                board_move.into_payload(),
            )?;
        }
        log_activity_payload(
            &tx,
            &state.id,
            actor,
            "update",
            serde_json::json!({
                "detail": key,
                "undo": undo,
            }),
        )?;
        tx.commit()?;

        self.card_by_id(&state.id)?
            .ok_or_else(|| anyhow!("card disappeared after update"))
    }
}

pub(super) struct CardUpdateState {
    pub(super) id: String,
    updated_at: i64,
    pub(super) claimed_by: Option<String>,
    pub(super) lease_expires_at: Option<i64>,
}

fn load_card_update_state(
    tx: &Transaction<'_>,
    board_id: &str,
    key: &str,
) -> Result<CardUpdateState> {
    tx.query_row(
        "SELECT id, updated_at, claimed_by, lease_expires_at FROM cards WHERE board_id = ?1 AND key_text = ?2",
        params![board_id, key],
        |r| {
            Ok(CardUpdateState {
                id: r.get(0)?,
                updated_at: r.get(1)?,
                claimed_by: r.get(2)?,
                lease_expires_at: r.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("no card '{key}'"))
}

fn checked_update_timestamp(state: &CardUpdateState, patch: &CardPatch, key: &str) -> Result<i64> {
    if let Some(expected) = patch.expected_updated_at {
        if expected != state.updated_at {
            return Err(anyhow!(
                "stale update for '{key}': expected updated_at={expected}, actual={}",
                state.updated_at
            ));
        }
    }
    let mut ts = now_ms();
    if ts <= state.updated_at {
        ts = state.updated_at + 1;
    }
    Ok(ts)
}
