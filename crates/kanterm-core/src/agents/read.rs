use anyhow::Result;
use rusqlite::params;

use crate::AgentRegistration;

use super::agent_from_row;

pub(super) fn agent_by_identity(
    conn: &rusqlite::Connection,
    assigned_identity: &str,
) -> Result<Option<AgentRegistration>> {
    let mut stmt = conn.prepare(
        "SELECT id, requested_name, assigned_identity, fingerprint_json, registered_at,
                last_seen_at, expires_at
           FROM agent_registrations
          WHERE assigned_identity = ?1",
    )?;
    let mut rows = stmt.query(params![assigned_identity])?;
    Ok(rows.next()?.map(agent_from_row).transpose()?)
}
