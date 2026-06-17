use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::id::new_id;
use crate::naming::derive_slug;
use crate::{now_ms, AgentRegistration, AgentRegistrationResult, Store};

const DEFAULT_AGENT_LEASE_MINUTES: i64 = 2 * 60;
const MAX_AGENT_LEASE_MINUTES: i64 = 24 * 60;
const MS_PER_MINUTE: i64 = 60 * 1000;

impl Store {
    pub fn register_agent(
        &mut self,
        requested_name: &str,
        remembered_identity: Option<&str>,
        fingerprint_json: Option<&str>,
        lease_minutes: Option<i64>,
    ) -> Result<AgentRegistrationResult> {
        self.assert_writable()?;
        let requested_name = normalize_requested_name(requested_name)?;
        let ts = now_ms();
        let expires_at = ts
            + lease_minutes
                .unwrap_or(DEFAULT_AGENT_LEASE_MINUTES)
                .clamp(1, MAX_AGENT_LEASE_MINUTES)
                * MS_PER_MINUTE;
        let token = random_token();
        let token_hash = hash_token(&token);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let registration = if let Some(identity) = remembered_identity.and_then(trimmed_optional) {
            rotate_registration(
                &tx,
                &requested_name,
                identity,
                &token_hash,
                fingerprint_json,
                ts,
                expires_at,
            )?
        } else {
            create_registration(
                &tx,
                &requested_name,
                &token_hash,
                fingerprint_json,
                ts,
                expires_at,
            )?
        };
        tx.commit()?;

        Ok(AgentRegistrationResult {
            registration,
            claim_token: token,
        })
    }

    pub fn agent_by_identity(&self, assigned_identity: &str) -> Result<Option<AgentRegistration>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, requested_name, assigned_identity, fingerprint_json, registered_at,
                    last_seen_at, expires_at
               FROM agent_registrations
              WHERE assigned_identity = ?1",
        )?;
        let mut rows = stmt.query(params![assigned_identity])?;
        Ok(rows.next()?.map(agent_from_row).transpose()?)
    }
}

pub(crate) fn validate_agent_token(
    tx: &Transaction<'_>,
    assigned_identity: &str,
    token: Option<&str>,
    ts: i64,
) -> Result<()> {
    let token = token
        .and_then(trimmed_optional)
        .ok_or_else(|| anyhow!("claim_token is required for agent claim operations"))?;
    let Some((expected_hash, expires_at)) = tx
        .query_row(
            "SELECT token_hash, expires_at
               FROM agent_registrations
              WHERE assigned_identity = ?1",
            params![assigned_identity],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )
        .optional()?
    else {
        return Err(anyhow!(
            "agent identity '{assigned_identity}' is not registered; call register_agent first"
        ));
    };
    if expires_at <= ts {
        return Err(anyhow!(
            "agent identity '{assigned_identity}' expired at {expires_at}; call register_agent again"
        ));
    }
    if !constant_time_eq(expected_hash.as_bytes(), hash_token(token).as_bytes()) {
        return Err(anyhow!(
            "claim_token did not verify for '{assigned_identity}'"
        ));
    }
    tx.execute(
        "UPDATE agent_registrations SET last_seen_at = ?1 WHERE assigned_identity = ?2",
        params![ts, assigned_identity],
    )?;
    Ok(())
}

fn create_registration(
    tx: &Transaction<'_>,
    requested_name: &str,
    token_hash: &str,
    fingerprint_json: Option<&str>,
    ts: i64,
    expires_at: i64,
) -> Result<AgentRegistration> {
    for _ in 0..16 {
        let id = new_id();
        let suffix = short_suffix(&id);
        let assigned_identity = format!("{}#{suffix}", derive_slug(requested_name));
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO agent_registrations
             (id, requested_name, assigned_identity, token_hash, fingerprint_json,
              registered_at, last_seen_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
            params![
                id,
                requested_name,
                assigned_identity,
                token_hash,
                fingerprint_json.and_then(trimmed_optional),
                ts,
                expires_at
            ],
        )?;
        if inserted == 1 {
            return Ok(AgentRegistration {
                id,
                requested_name: requested_name.to_string(),
                assigned_identity,
                fingerprint_json: fingerprint_json
                    .and_then(trimmed_optional)
                    .map(str::to_string),
                registered_at: ts,
                last_seen_at: ts,
                expires_at,
            });
        }
    }
    Err(anyhow!("could not allocate a unique agent identity"))
}

fn rotate_registration(
    tx: &Transaction<'_>,
    requested_name: &str,
    assigned_identity: &str,
    token_hash: &str,
    fingerprint_json: Option<&str>,
    ts: i64,
    expires_at: i64,
) -> Result<AgentRegistration> {
    let updated = tx.execute(
        "UPDATE agent_registrations
            SET requested_name = ?1,
                token_hash = ?2,
                fingerprint_json = ?3,
                last_seen_at = ?4,
                expires_at = ?5
          WHERE assigned_identity = ?6",
        params![
            requested_name,
            token_hash,
            fingerprint_json.and_then(trimmed_optional),
            ts,
            expires_at,
            assigned_identity
        ],
    )?;
    if updated == 0 {
        return Err(anyhow!(
            "remembered agent identity '{assigned_identity}' is not registered"
        ));
    }
    tx.query_row(
        "SELECT id, requested_name, assigned_identity, fingerprint_json, registered_at,
                last_seen_at, expires_at
           FROM agent_registrations
          WHERE assigned_identity = ?1",
        params![assigned_identity],
        agent_from_row,
    )
    .map_err(Into::into)
}

fn agent_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentRegistration> {
    Ok(AgentRegistration {
        id: row.get(0)?,
        requested_name: row.get(1)?,
        assigned_identity: row.get(2)?,
        fingerprint_json: row.get(3)?,
        registered_at: row.get(4)?,
        last_seen_at: row.get(5)?,
        expires_at: row.get(6)?,
    })
}

fn normalize_requested_name(name: &str) -> Result<String> {
    trimmed_optional(name)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("requested_name is required"))
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).expect("operating system randomness is unavailable");
    hex(&bytes)
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("sha256:{}", hex(&digest))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn short_suffix(id: &str) -> String {
    id.chars()
        .rev()
        .take(6)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn trimmed_optional(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

trait OptionalRow<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRow<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err),
        }
    }
}
