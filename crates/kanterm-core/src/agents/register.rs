use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::id::new_id;
use crate::naming::derive_slug;
use crate::{now_ms, AgentRegistration, AgentRegistrationResult, Store};

use super::{agent_from_row, lease_expires_at, token, trimmed_optional};

pub(super) fn register_agent(
    store: &mut Store,
    requested_name: &str,
    remembered_identity: Option<&str>,
    fingerprint_json: Option<&str>,
    lease_minutes: Option<i64>,
) -> Result<AgentRegistrationResult> {
    store.assert_writable()?;
    let requested_name = normalize_requested_name(requested_name)?;
    let ts = now_ms();
    let expires_at = lease_expires_at(ts, lease_minutes);
    let claim_token = token::random_token();
    let token_hash = token::hash_token(&claim_token);
    let tx = store
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
        claim_token,
    })
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

fn normalize_requested_name(name: &str) -> Result<String> {
    trimmed_optional(name)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("requested_name is required"))
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
