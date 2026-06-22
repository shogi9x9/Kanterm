use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::{parse_date, CardPatch};

pub(in crate::cards) fn apply_scalar_fields(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
    ts: i64,
) -> Result<()> {
    if let Some(title) = &patch.title {
        tx.execute(
            "UPDATE cards SET title = ?1 WHERE id = ?2",
            params![title, card_id],
        )?;
    }
    if let Some(body) = &patch.body {
        tx.execute(
            "UPDATE cards SET body = ?1 WHERE id = ?2",
            params![body, card_id],
        )?;
    }
    if let Some(agent_state) = &patch.agent_state {
        tx.execute(
            "UPDATE cards SET status = ?1 WHERE id = ?2",
            params![agent_state, card_id],
        )?;
    }
    if let Some(priority) = patch.priority {
        tx.execute(
            "UPDATE cards SET priority = ?1 WHERE id = ?2",
            params![priority, card_id],
        )?;
    }
    if patch.assignee.is_some() {
        tx.execute(
            "UPDATE cards SET assignee = ?1 WHERE id = ?2",
            params![patch.assignee, card_id],
        )?;
    }
    if let Some(archived) = patch.archived {
        let val = if archived { Some(ts) } else { None };
        tx.execute(
            "UPDATE cards SET archived_at = ?1 WHERE id = ?2",
            params![val, card_id],
        )?;
    }
    Ok(())
}

pub(in crate::cards) fn apply_due_date(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(due) = &patch.due {
        let val: Option<i64> = if due.trim().is_empty() {
            None
        } else {
            Some(parse_date(due)?)
        };
        tx.execute(
            "UPDATE cards SET due_date = ?1 WHERE id = ?2",
            params![val, card_id],
        )?;
    }
    Ok(())
}
