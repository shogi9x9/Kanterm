use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};

use super::trimmed_optional;

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

pub(super) fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).expect("operating system randomness is unavailable");
    hex(&bytes)
}

pub(super) fn hash_token(token: &str) -> String {
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

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}
