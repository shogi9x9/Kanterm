use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use super::AgentTaskAttempt;
use crate::id::new_id;
use crate::{now_ms, Store};

impl Store {
    pub fn start_agent_task_attempt(
        &mut self,
        handoff_id: &str,
        target_name: &str,
        packet_version: &str,
        packet_profile: &str,
        packet_text: &str,
    ) -> Result<AgentTaskAttempt> {
        self.assert_writable()?;
        let target_name = required(target_name, "target_name")?;
        let packet_version = required(packet_version, "packet_version")?;
        let packet_profile = required(packet_profile, "packet_profile")?;
        if packet_text.is_empty() {
            return Err(anyhow!("packet_text is required"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let handoff_exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM agent_handoffs WHERE id = ?1)",
            params![handoff_id],
            |row| row.get(0),
        )?;
        if !handoff_exists {
            return Err(anyhow!("no handoff '{handoff_id}'"));
        }
        let attempt_no: i64 = tx.query_row(
            "SELECT COALESCE(MAX(attempt_no), 0) + 1
               FROM agent_task_attempts WHERE handoff_id = ?1",
            params![handoff_id],
            |row| row.get(0),
        )?;
        let id = new_id();
        let started_at = now_ms();
        let packet_sha256 = format!("{:x}", Sha256::digest(packet_text.as_bytes()));
        tx.execute(
            "INSERT INTO agent_task_attempts
             (id, handoff_id, attempt_no, target_name, packet_version, packet_profile,
              packet_sha256, packet_text, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'running', ?9)",
            params![
                id,
                handoff_id,
                attempt_no,
                target_name,
                packet_version,
                packet_profile,
                packet_sha256,
                packet_text,
                started_at
            ],
        )?;
        let attempt = load_attempt_tx(&tx, &id)?;
        tx.commit()?;
        Ok(attempt)
    }

    pub fn finish_agent_task_attempt(
        &mut self,
        id: &str,
        status: &str,
        note: Option<&str>,
    ) -> Result<AgentTaskAttempt> {
        self.assert_writable()?;
        if !matches!(status, "agent_succeeded" | "agent_failed") {
            return Err(anyhow!(
                "attempt status must be agent_succeeded or agent_failed"
            ));
        }
        let completed_at = now_ms();
        let (agent_output, error_text) = if status == "agent_succeeded" {
            (note, None)
        } else {
            (None, note)
        };
        let changed = self.conn.execute(
            "UPDATE agent_task_attempts
                SET status = ?1, agent_output = ?2, error_text = ?3, completed_at = ?4
              WHERE id = ?5 AND status = 'running'",
            params![status, agent_output, error_text, completed_at, id],
        )?;
        if changed == 0 {
            return Err(anyhow!("attempt '{id}' is not running"));
        }
        self.agent_task_attempt_by_id(id)?
            .ok_or_else(|| anyhow!("updated attempt '{id}' could not be loaded"))
    }

    pub fn agent_task_attempts(&self, handoff_id: &str) -> Result<Vec<AgentTaskAttempt>> {
        let mut statement = self.conn.prepare(
            "SELECT id, handoff_id, attempt_no, target_name, packet_version, packet_profile,
                    packet_sha256, packet_text, status, agent_output, error_text,
                    started_at, completed_at
               FROM agent_task_attempts
              WHERE handoff_id = ?1
              ORDER BY attempt_no",
        )?;
        let attempts = statement
            .query_map(params![handoff_id], row_to_attempt)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(attempts)
    }

    fn agent_task_attempt_by_id(&self, id: &str) -> Result<Option<AgentTaskAttempt>> {
        self.conn
            .query_row(
                "SELECT id, handoff_id, attempt_no, target_name, packet_version, packet_profile,
                        packet_sha256, packet_text, status, agent_output, error_text,
                        started_at, completed_at
                   FROM agent_task_attempts WHERE id = ?1",
                params![id],
                row_to_attempt,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn load_attempt_tx(tx: &Transaction<'_>, id: &str) -> Result<AgentTaskAttempt> {
    tx.query_row(
        "SELECT id, handoff_id, attempt_no, target_name, packet_version, packet_profile,
                packet_sha256, packet_text, status, agent_output, error_text,
                started_at, completed_at
           FROM agent_task_attempts WHERE id = ?1",
        params![id],
        row_to_attempt,
    )
    .map_err(Into::into)
}

fn row_to_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentTaskAttempt> {
    Ok(AgentTaskAttempt {
        id: row.get(0)?,
        handoff_id: row.get(1)?,
        attempt_no: row.get(2)?,
        target_name: row.get(3)?,
        packet_version: row.get(4)?,
        packet_profile: row.get(5)?,
        packet_sha256: row.get(6)?,
        packet_text: row.get(7)?,
        status: row.get(8)?,
        agent_output: row.get(9)?,
        error_text: row.get(10)?,
        started_at: row.get(11)?,
        completed_at: row.get(12)?,
    })
}

fn required<'a>(value: &'a str, field: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("{field} is required"));
    }
    Ok(value)
}
