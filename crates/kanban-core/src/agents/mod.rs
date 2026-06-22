use crate::{AgentRegistration, AgentRegistrationResult, Store};
use anyhow::Result;

mod read;
mod register;
mod token;

pub(crate) use token::validate_agent_token;

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
        register::register_agent(
            self,
            requested_name,
            remembered_identity,
            fingerprint_json,
            lease_minutes,
        )
    }

    pub fn agent_by_identity(&self, assigned_identity: &str) -> Result<Option<AgentRegistration>> {
        read::agent_by_identity(&self.conn, assigned_identity)
    }
}

fn lease_expires_at(ts: i64, lease_minutes: Option<i64>) -> i64 {
    ts + lease_minutes
        .unwrap_or(DEFAULT_AGENT_LEASE_MINUTES)
        .clamp(1, MAX_AGENT_LEASE_MINUTES)
        * MS_PER_MINUTE
}

fn trimmed_optional(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
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
