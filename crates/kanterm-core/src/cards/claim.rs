use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};

use crate::agents::validate_agent_token;
use crate::text::trimmed_optional;
use crate::CardPatch;

use super::update::CardUpdateState;

const MS_PER_MINUTE: i64 = 60_000;
const DEFAULT_LEASE_MINUTES: i64 = 60;
const MAX_LEASE_MINUTES: i64 = 24 * 60;

pub(super) fn apply_claim_patch(
    tx: &Transaction<'_>,
    state: &CardUpdateState,
    patch: &CardPatch,
    key: &str,
    ts: i64,
) -> Result<()> {
    if patch.release_claim == Some(true) || patch.claim.as_deref().map(str::trim) == Some("") {
        if let Some(existing) = state.claimed_by.as_deref() {
            validate_agent_token(tx, existing, patch.claim_token.as_deref(), ts)?;
        }
        tx.execute(
            "UPDATE cards SET claimed_by = NULL, claimed_at = NULL, lease_expires_at = NULL WHERE id = ?1",
            params![state.id],
        )?;
    } else if patch.archived == Some(true) && patch.claim.is_none() {
        tx.execute(
            "UPDATE cards SET claimed_by = NULL, claimed_at = NULL, lease_expires_at = NULL WHERE id = ?1",
            params![state.id],
        )?;
    } else if let Some(claimant) = patch.claim.as_deref().and_then(trimmed_optional) {
        validate_agent_token(tx, claimant, patch.claim_token.as_deref(), ts)?;
        if let (Some(existing), Some(expires_at)) =
            (state.claimed_by.as_deref(), state.lease_expires_at)
        {
            if existing != claimant && expires_at > ts {
                return Err(anyhow!(
                    "card '{key}' is claimed by '{existing}' until lease_expires_at={expires_at}"
                ));
            }
        }
        let minutes = patch
            .lease_minutes
            .unwrap_or(DEFAULT_LEASE_MINUTES)
            .clamp(1, MAX_LEASE_MINUTES);
        let lease_expires_at = ts + minutes * MS_PER_MINUTE;
        tx.execute(
            "UPDATE cards SET claimed_by = ?1, claimed_at = ?2, lease_expires_at = ?3 WHERE id = ?4",
            params![claimant, ts, lease_expires_at, state.id],
        )?;
    }
    Ok(())
}
