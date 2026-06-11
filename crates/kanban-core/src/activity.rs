use anyhow::Result;
use rusqlite::params;

use crate::{now_ms, ActivityLog, Store};

pub(crate) fn log_activity(
    tx: &rusqlite::Transaction,
    card_id: &str,
    actor: &str,
    action: &str,
    detail: &str,
) -> Result<()> {
    log_activity_payload(
        tx,
        card_id,
        actor,
        action,
        serde_json::json!({ "detail": detail }),
    )
}

pub(crate) fn log_activity_payload(
    tx: &rusqlite::Transaction,
    card_id: &str,
    actor: &str,
    action: &str,
    payload: serde_json::Value,
) -> Result<()> {
    tx.execute(
        "INSERT INTO activity_logs (card_id, actor, action, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![card_id, actor, action, payload.to_string(), now_ms()],
    )?;
    Ok(())
}

impl Store {
    pub fn record_execution_note(
        &mut self,
        board_id: &str,
        key: &str,
        note: &str,
        actor: &str,
    ) -> Result<()> {
        self.assert_writable()?;
        let note = note.trim();
        if note.is_empty() {
            return Ok(());
        }
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let card_id: String = tx.query_row(
            "SELECT id FROM cards WHERE board_id = ?1 AND key_text = ?2",
            params![board_id, key],
            |r| r.get(0),
        )?;
        log_activity_payload(
            &tx,
            &card_id,
            actor,
            "execution_note",
            serde_json::json!({ "note": note }),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn card_activity(&self, card_id: &str, limit: i64) -> Result<Vec<ActivityLog>> {
        let limit = limit.clamp(1, 50);
        let mut stmt = self.conn.prepare(
            "SELECT actor, action, payload_json, created_at
             FROM activity_logs
             WHERE card_id = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![card_id, limit], |r| {
                Ok(ActivityLog {
                    actor: r.get(0)?,
                    action: r.get(1)?,
                    payload_json: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}
