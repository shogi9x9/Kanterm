use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::agents::validate_agent_token;
use crate::id::new_id;
use crate::naming::derive_slug;
use crate::{now_ms, AgentHandoff, HandoffDraft, HandoffListQuery, HandoffStatusPatch, Store};

const DEFAULT_HANDOFF_LEASE_MINUTES: i64 = 60;
const MAX_HANDOFF_LEASE_MINUTES: i64 = 24 * 60;
const MS_PER_MINUTE: i64 = 60 * 1000;

impl Store {
    pub fn create_handoff(&mut self, draft: &HandoffDraft) -> Result<AgentHandoff> {
        self.assert_writable()?;
        let from_agent = required(&draft.from_agent, "from_agent")?;
        let to_agent = required(&draft.to_agent, "to_agent")?;
        let subject = required(&draft.subject, "subject")?;
        let body = required(&draft.body, "body")?;
        let card_key = draft.card_key.as_deref().and_then(trimmed_optional);
        if let (Some(board_id), Some(key)) = (draft.board_id.as_deref(), card_key) {
            let exists: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM cards WHERE board_id = ?1 AND key_text = ?2",
                    params![board_id, key],
                    |r| r.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(anyhow!("no card '{key}' on selected board"));
            }
        }
        let ts = now_ms();
        let id = new_id();
        self.conn.execute(
            "INSERT INTO agent_handoffs
             (id, from_agent, to_agent, board_id, card_key, subject, body, status,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?8)",
            params![
                id,
                from_agent,
                to_agent,
                draft.board_id.as_deref(),
                card_key,
                subject,
                body,
                ts
            ],
        )?;
        self.handoff_by_id(&id)?
            .ok_or_else(|| anyhow!("created handoff could not be loaded"))
    }

    pub fn list_handoffs(&self, query: HandoffListQuery<'_>) -> Result<Vec<AgentHandoff>> {
        let limit = query.limit.clamp(1, 100);
        let recipient = query.recipient.and_then(trimmed_optional);
        let sender = query.sender.and_then(trimmed_optional);
        let status = query.status.map(normalize_status_filter).transpose()?;
        let recipient_base = recipient
            .and_then(|value| value.split_once('#').map(|(base, _)| base.to_string()))
            .unwrap_or_default();
        let closed_clause = if query.include_closed || status.is_some() {
            ""
        } else {
            "AND status IN ('pending', 'claimed')"
        };
        let ts = now_ms();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, from_agent, to_agent, board_id, card_key, subject, body, status,
                    claimed_by, claimed_at, lease_expires_at, completed_at, failed_at,
                    result_text, last_error, created_at, updated_at
               FROM agent_handoffs
              WHERE (?1 IS NULL OR to_agent = ?1 OR to_agent = ?2)
                    AND (?3 IS NULL OR from_agent = ?3)
                    AND (?4 IS NULL OR status = ?4)
                    {closed_clause}
                    AND (?6 = 0 OR status = 'pending'
                         OR (status = 'claimed' AND lease_expires_at <= ?7))
              ORDER BY created_at ASC
              LIMIT ?5"
        ))?;
        let rows = stmt
            .query_map(
                params![
                    recipient,
                    recipient_base,
                    sender,
                    status,
                    limit,
                    query.claimable_only,
                    ts
                ],
                row_to_handoff,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn handoff_by_id(&self, id: &str) -> Result<Option<AgentHandoff>> {
        self.conn
            .query_row(
                "SELECT id, from_agent, to_agent, board_id, card_key, subject, body, status,
                        claimed_by, claimed_at, lease_expires_at, completed_at, failed_at,
                        result_text, last_error, created_at, updated_at
                   FROM agent_handoffs
                  WHERE id = ?1",
                params![id],
                row_to_handoff,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn claim_handoff(
        &mut self,
        id: &str,
        claimant: &str,
        claim_token: Option<&str>,
        lease_minutes: Option<i64>,
    ) -> Result<AgentHandoff> {
        self.assert_writable()?;
        let claimant = required(claimant, "claimant")?;
        let ts = now_ms();
        let lease_expires_at = lease_expires_at(ts, lease_minutes);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_agent_token(&tx, claimant, claim_token, ts)?;
        let handoff = load_handoff_tx(&tx, id)?;
        validate_recipient(&handoff, claimant)?;
        match handoff.status.as_str() {
            "pending" => {}
            "claimed" => {
                let active_other_claim = handoff.claimed_by.as_deref() != Some(claimant)
                    && handoff
                        .lease_expires_at
                        .map(|expires_at| expires_at > ts)
                        .unwrap_or(false);
                if active_other_claim {
                    return Err(anyhow!(
                        "handoff '{}' is claimed by '{}' until lease_expires_at={}",
                        handoff.id,
                        handoff.claimed_by.as_deref().unwrap_or("-"),
                        handoff.lease_expires_at.unwrap_or_default()
                    ));
                }
            }
            other => return Err(anyhow!("handoff '{}' is already {other}", handoff.id)),
        }
        tx.execute(
            "UPDATE agent_handoffs
                SET status = 'claimed',
                    claimed_by = ?1,
                    claimed_at = ?2,
                    lease_expires_at = ?3,
                    updated_at = ?2
              WHERE id = ?4",
            params![claimant, ts, lease_expires_at, id],
        )?;
        let updated = load_handoff_tx(&tx, id)?;
        tx.commit()?;
        Ok(updated)
    }

    pub fn update_handoff_status(
        &mut self,
        id: &str,
        claimant: &str,
        claim_token: Option<&str>,
        patch: &HandoffStatusPatch,
    ) -> Result<AgentHandoff> {
        self.assert_writable()?;
        let claimant = required(claimant, "claimant")?;
        let status = normalize_terminal_status(&patch.status)?;
        let note = patch.note.as_deref().and_then(trimmed_optional);
        let ts = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_agent_token(&tx, claimant, claim_token, ts)?;
        let handoff = load_handoff_tx(&tx, id)?;
        if handoff.claimed_by.as_deref() != Some(claimant) {
            return Err(anyhow!(
                "handoff '{}' must be claimed by '{claimant}' before completion",
                handoff.id
            ));
        }
        if handoff.status != "claimed" {
            return Err(anyhow!(
                "handoff '{}' must be claimed before completion",
                handoff.id
            ));
        }
        let (completed_at, failed_at, result_text, last_error) = if status == "completed" {
            (Some(ts), None, note, None)
        } else {
            (None, Some(ts), None, note)
        };
        tx.execute(
            "UPDATE agent_handoffs
                SET status = ?1,
                    completed_at = ?2,
                    failed_at = ?3,
                    result_text = ?4,
                    last_error = ?5,
                    updated_at = ?6
              WHERE id = ?7",
            params![
                status,
                completed_at,
                failed_at,
                result_text,
                last_error,
                ts,
                id
            ],
        )?;
        let updated = load_handoff_tx(&tx, id)?;
        tx.commit()?;
        Ok(updated)
    }

    pub fn requeue_handoff(
        &mut self,
        id: &str,
        claimant: &str,
        claim_token: Option<&str>,
        note: Option<&str>,
    ) -> Result<AgentHandoff> {
        self.assert_writable()?;
        let claimant = required(claimant, "claimant")?;
        let note = note.and_then(trimmed_optional);
        let ts = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_agent_token(&tx, claimant, claim_token, ts)?;
        let handoff = load_handoff_tx(&tx, id)?;
        if handoff.claimed_by.as_deref() != Some(claimant) || handoff.status != "claimed" {
            return Err(anyhow!(
                "handoff '{}' must be claimed by '{claimant}' before requeue",
                handoff.id
            ));
        }
        tx.execute(
            "UPDATE agent_handoffs
                SET status = 'pending',
                    claimed_by = NULL,
                    claimed_at = NULL,
                    lease_expires_at = NULL,
                    last_error = ?1,
                    updated_at = ?2
              WHERE id = ?3",
            params![note, ts, id],
        )?;
        let updated = load_handoff_tx(&tx, id)?;
        tx.commit()?;
        Ok(updated)
    }
}

fn load_handoff_tx(tx: &Transaction<'_>, id: &str) -> Result<AgentHandoff> {
    tx.query_row(
        "SELECT id, from_agent, to_agent, board_id, card_key, subject, body, status,
                claimed_by, claimed_at, lease_expires_at, completed_at, failed_at,
                result_text, last_error, created_at, updated_at
           FROM agent_handoffs
          WHERE id = ?1",
        params![id],
        row_to_handoff,
    )
    .map_err(Into::into)
}

fn row_to_handoff(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentHandoff> {
    Ok(AgentHandoff {
        id: row.get(0)?,
        from_agent: row.get(1)?,
        to_agent: row.get(2)?,
        board_id: row.get(3)?,
        card_key: row.get(4)?,
        subject: row.get(5)?,
        body: row.get(6)?,
        status: row.get(7)?,
        claimed_by: row.get(8)?,
        claimed_at: row.get(9)?,
        lease_expires_at: row.get(10)?,
        completed_at: row.get(11)?,
        failed_at: row.get(12)?,
        result_text: row.get(13)?,
        last_error: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn validate_recipient(handoff: &AgentHandoff, claimant: &str) -> Result<()> {
    if handoff.to_agent == claimant {
        return Ok(());
    }
    if !handoff.to_agent.contains('#') {
        let expected = derive_slug(&handoff.to_agent);
        let actual = claimant.split_once('#').map(|(base, _)| base);
        if actual == Some(expected.as_str()) {
            return Ok(());
        }
    }
    Err(anyhow!(
        "handoff '{}' is addressed to '{}', not '{claimant}'",
        handoff.id,
        handoff.to_agent
    ))
}

fn normalize_terminal_status(status: &str) -> Result<&'static str> {
    match required(status, "status")? {
        "completed" | "done" => Ok("completed"),
        "failed" | "error" => Ok("failed"),
        _ => Err(anyhow!("status must be completed or failed")),
    }
}

fn normalize_status_filter(status: &str) -> Result<&'static str> {
    match required(status, "status")? {
        "pending" => Ok("pending"),
        "claimed" => Ok("claimed"),
        "completed" | "done" => Ok("completed"),
        "failed" | "error" => Ok("failed"),
        _ => Err(anyhow!(
            "status filter must be pending, claimed, completed, or failed"
        )),
    }
}

fn lease_expires_at(ts: i64, lease_minutes: Option<i64>) -> i64 {
    ts + lease_minutes
        .unwrap_or(DEFAULT_HANDOFF_LEASE_MINUTES)
        .clamp(1, MAX_HANDOFF_LEASE_MINUTES)
        * MS_PER_MINUTE
}

fn required<'a>(value: &'a str, name: &str) -> Result<&'a str> {
    trimmed_optional(value).ok_or_else(|| anyhow!("{name} is required"))
}

fn trimmed_optional(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
