use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::text::trimmed_optional;
use crate::CardPatch;

pub(in crate::cards) fn apply_workflow_fields(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(next_action) = &patch.next_action {
        tx.execute(
            "UPDATE cards SET next_action = ?1 WHERE id = ?2",
            params![trimmed_optional(next_action), card_id],
        )?;
    }
    if let Some(blocked_reason) = &patch.blocked_reason {
        tx.execute(
            "UPDATE cards SET blocked_reason = ?1 WHERE id = ?2",
            params![trimmed_optional(blocked_reason), card_id],
        )?;
    }
    if let Some(acceptance_criteria) = &patch.acceptance_criteria {
        tx.execute(
            "UPDATE cards SET acceptance_criteria = ?1 WHERE id = ?2",
            params![trimmed_optional(acceptance_criteria), card_id],
        )?;
    }
    if let Some(handoff_note) = &patch.handoff_note {
        tx.execute(
            "UPDATE cards SET handoff_note = ?1 WHERE id = ?2",
            params![trimmed_optional(handoff_note), card_id],
        )?;
    }
    if let Some(last_verification) = &patch.last_verification {
        tx.execute(
            "UPDATE cards SET last_verification = ?1 WHERE id = ?2",
            params![trimmed_optional(last_verification), card_id],
        )?;
    }
    Ok(())
}
