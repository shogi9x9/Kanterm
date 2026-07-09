use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::activity::log_activity_payload;
use crate::search::sync_card_search_row;
use crate::{now_ms, Card, Store};

impl Store {
    /// Undo the latest card update currently visible on a board. This is a
    /// one-step safety net for accidental archive/complete/move/edit actions;
    /// hard deletes and board/column structure changes remain intentionally
    /// irreversible.
    pub fn undo_last_card_update(&mut self, board_id: &str, actor: &str) -> Result<Option<Card>> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let last_undo_id: i64 = tx.query_row(
            "SELECT COALESCE(MAX(id), 0) FROM activity_logs WHERE action = 'undo'",
            [],
            |r| r.get(0),
        )?;
        let mut stmt = tx.prepare(
            "SELECT al.id, al.card_id, al.payload_json
               FROM activity_logs al
               JOIN cards c ON c.id = al.card_id
              WHERE c.board_id = ?1
                AND al.action = 'update'
                AND al.id > ?2
              ORDER BY al.id DESC
              LIMIT 50",
        )?;
        let candidates = stmt
            .query_map(params![board_id, last_undo_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let Some((_, card_id, undo)) = candidates.into_iter().find_map(|(id, card_id, payload)| {
            let payload = serde_json::from_str::<serde_json::Value>(&payload).ok()?;
            let undo = serde_json::from_value::<Card>(payload.get("undo")?.clone()).ok()?;
            Some((id, card_id, undo))
        }) else {
            tx.commit()?;
            return Ok(None);
        };

        restore_card_snapshot(&tx, &undo)?;
        sync_card_search_row(&tx, &card_id)?;
        log_activity_payload(
            &tx,
            &card_id,
            actor,
            "undo",
            serde_json::json!({
                "detail": format!("restored {}", undo.key),
            }),
        )?;
        tx.commit()?;

        self.card_by_id(&card_id)?
            .map(Some)
            .ok_or_else(|| anyhow!("card disappeared after undo"))
    }
}

fn restore_card_snapshot(tx: &Transaction<'_>, card: &Card) -> Result<()> {
    tx.execute(
        "UPDATE cards
            SET board_id = ?1,
                column_id = ?2,
                key_text = ?3,
                title = ?4,
                body = ?5,
                status = ?6,
                priority = ?7,
                assignee = ?8,
                due_date = ?9,
                next_action = ?10,
                blocked_reason = ?11,
                acceptance_criteria = ?12,
                handoff_note = ?13,
                last_verification = ?14,
                agent_weight = ?15,
                agent_effort = ?16,
                suggested_model = ?17,
                expected_tokens = ?18,
                human_intervention = ?19,
                claimed_by = ?20,
                claimed_at = ?21,
                lease_expires_at = ?22,
                position = ?23,
                updated_at = ?24,
                archived_at = ?25
          WHERE id = ?26",
        params![
            &card.board_id,
            &card.column_id,
            &card.key,
            &card.title,
            &card.body,
            &card.agent_state,
            card.priority,
            &card.assignee,
            card.due_date,
            &card.next_action,
            &card.blocked_reason,
            &card.acceptance_criteria,
            &card.handoff_note,
            &card.last_verification,
            card.agent_weight,
            &card.agent_effort,
            &card.suggested_model,
            card.expected_tokens,
            &card.human_intervention,
            &card.claimed_by,
            card.claimed_at,
            card.lease_expires_at,
            card.position,
            now_ms().max(card.updated_at + 1),
            card.archived_at,
            &card.id,
        ],
    )?;
    Ok(())
}
