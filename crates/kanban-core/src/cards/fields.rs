use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::labels::upsert_label;
use crate::position::next_position;
use crate::text::trimmed_optional;
use crate::{parse_date, CardPatch};

pub(super) fn apply_scalar_fields(
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

pub(super) fn apply_column_move(
    tx: &Transaction<'_>,
    board_id: &str,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(column_name) = &patch.column {
        let column_id: String = tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 AND name = ?2",
                params![board_id, column_name],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no column named '{column_name}'"))?;
        let position = next_position(tx, &column_id)?;
        tx.execute(
            "UPDATE cards SET column_id = ?1, position = ?2 WHERE id = ?3",
            params![column_id, position, card_id],
        )?;
    }
    Ok(())
}

pub(super) fn apply_board_rehome(
    tx: &Transaction<'_>,
    source_board_id: &str,
    target_board_id: &mut String,
    card_id: &str,
    old_key: &str,
    patch: &CardPatch,
) -> Result<Option<BoardMoveActivity>> {
    let Some(dest_board_id) = &patch.move_to_board else {
        return Ok(None);
    };
    if dest_board_id == source_board_id {
        return Ok(None);
    }

    let (source_name, source_slug): (String, String) = tx
        .query_row(
            "SELECT name, slug FROM boards WHERE id = ?1",
            params![source_board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no board '{source_board_id}'"))?;

    let (dest_name, dest_slug, prefix, seq): (String, String, String, i64) = tx
        .query_row(
            "SELECT name, slug, key_prefix, card_seq FROM boards WHERE id = ?1",
            params![dest_board_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no board '{dest_board_id}'"))?;

    let new_key = format!("{}-{}", prefix, seq + 1);
    tx.execute(
        "UPDATE boards SET card_seq = ?1 WHERE id = ?2",
        params![seq + 1, dest_board_id],
    )?;

    let dest_col: String = tx
        .query_row(
            "SELECT id FROM columns WHERE board_id = ?1 ORDER BY sort_order LIMIT 1",
            params![dest_board_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("destination board has no columns"))?;

    tx.execute(
        "UPDATE cards SET board_id = ?1, column_id = ?2, position = ?3, key_text = ?4 WHERE id = ?5",
        params![
            dest_board_id,
            dest_col,
            next_position(tx, &dest_col)?,
            new_key,
            card_id
        ],
    )?;
    *target_board_id = dest_board_id.to_string();
    Ok(Some(BoardMoveActivity {
        old_key: old_key.to_string(),
        new_key,
        source_board_id: source_board_id.to_string(),
        source_board_name: source_name,
        source_board_slug: source_slug,
        destination_board_id: dest_board_id.to_string(),
        destination_board_name: dest_name,
        destination_board_slug: dest_slug,
    }))
}

#[derive(Debug)]
pub(super) struct BoardMoveActivity {
    old_key: String,
    new_key: String,
    source_board_id: String,
    source_board_name: String,
    source_board_slug: String,
    destination_board_id: String,
    destination_board_name: String,
    destination_board_slug: String,
}

impl BoardMoveActivity {
    pub(super) fn into_payload(self) -> serde_json::Value {
        serde_json::json!({
            "old_key": self.old_key,
            "new_key": self.new_key,
            "source_board": {
                "id": self.source_board_id,
                "name": self.source_board_name,
                "slug": self.source_board_slug,
            },
            "destination_board": {
                "id": self.destination_board_id,
                "name": self.destination_board_name,
                "slug": self.destination_board_slug,
            },
        })
    }
}

pub(super) fn apply_label_changes(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
    ts: i64,
) -> Result<()> {
    if let Some(names) = &patch.add_labels {
        for name in names {
            let label_id = upsert_label(tx, name)?;
            tx.execute(
                "INSERT OR IGNORE INTO card_labels (card_id, label_id) VALUES (?1, ?2)",
                params![card_id, label_id],
            )?;
            tx.execute(
                "UPDATE labels SET last_used_at = ?1 WHERE id = ?2",
                params![ts, label_id],
            )?;
        }
    }
    if let Some(names) = &patch.remove_labels {
        for name in names {
            tx.execute(
                "DELETE FROM card_labels WHERE card_id = ?1 AND label_id =
                   (SELECT id FROM labels WHERE name = ?2)",
                params![card_id, name],
            )?;
        }
    }
    Ok(())
}

pub(super) fn apply_due_date(tx: &Transaction<'_>, card_id: &str, patch: &CardPatch) -> Result<()> {
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

pub(super) fn apply_workflow_fields(
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

pub(super) fn apply_execution_metadata(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(agent_weight) = patch.agent_weight {
        if let Some(weight) = agent_weight {
            if !(1..=5).contains(&weight) {
                return Err(anyhow!("agent_weight must be between 1 and 5"));
            }
        }
        tx.execute(
            "UPDATE cards SET agent_weight = ?1 WHERE id = ?2",
            params![agent_weight, card_id],
        )?;
    }
    if let Some(agent_effort) = &patch.agent_effort {
        tx.execute(
            "UPDATE cards SET agent_effort = ?1 WHERE id = ?2",
            params![trimmed_optional(agent_effort), card_id],
        )?;
    }
    if let Some(suggested_model) = &patch.suggested_model {
        tx.execute(
            "UPDATE cards SET suggested_model = ?1 WHERE id = ?2",
            params![trimmed_optional(suggested_model), card_id],
        )?;
    }
    if let Some(expected_tokens) = patch.expected_tokens {
        if let Some(tokens) = expected_tokens {
            if tokens <= 0 {
                return Err(anyhow!("expected_tokens must be positive"));
            }
        }
        tx.execute(
            "UPDATE cards SET expected_tokens = ?1 WHERE id = ?2",
            params![expected_tokens, card_id],
        )?;
    }
    if let Some(human_intervention) = &patch.human_intervention {
        let value = trimmed_optional(human_intervention);
        if let Some(value) = value {
            match value {
                "none" | "review" | "decision" | "execution" => {}
                _ => {
                    return Err(anyhow!(
                        "human_intervention must be none, review, decision, or execution"
                    ))
                }
            }
        }
        tx.execute(
            "UPDATE cards SET human_intervention = ?1 WHERE id = ?2",
            params![value, card_id],
        )?;
    }
    Ok(())
}
