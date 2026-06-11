use kanban_core::Store;
use rmcp::ErrorData;

use crate::error::internal;
use crate::params::RegisterAgentParams;

pub(crate) fn register_agent(
    store: &mut Store,
    p: RegisterAgentParams,
) -> Result<String, ErrorData> {
    let result = store
        .register_agent(
            &p.requested_name,
            p.remembered_identity.as_deref(),
            p.fingerprint.as_deref(),
            p.lease_minutes,
        )
        .map_err(internal)?;
    Ok(format!(
        "assigned_identity: {}\nregistration_id: {}\nclaim_token: {}\nexpires_at: {}\n\nUse assigned_identity as update_card.claim and pass claim_token with claim/release operations. Store both outside volatile chat context when possible.",
        result.registration.assigned_identity,
        result.registration.id,
        result.claim_token,
        result.registration.expires_at
    ))
}
