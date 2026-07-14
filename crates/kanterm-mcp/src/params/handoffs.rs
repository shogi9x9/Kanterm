use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SendHandoffParams {
    /// Sender identity or display name.
    pub(crate) from_agent: String,
    /// Recipient identity, e.g. "claude#abc123", or family name, e.g. "claude".
    pub(crate) to_agent: String,
    /// Board slug for card context. Defaults to the server default board.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// Optional card key on the selected board.
    #[serde(default)]
    pub(crate) card: Option<String>,
    /// Short inbox title.
    pub(crate) subject: String,
    /// Work request or handoff body.
    pub(crate) body: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ListHandoffsParams {
    /// Recipient identity to read. When omitted, returns all open handoffs.
    #[serde(default)]
    pub(crate) for_agent: Option<String>,
    /// Filter by the sender identity/name.
    #[serde(default)]
    pub(crate) from_agent: Option<String>,
    /// Filter by pending, claimed, completed, or failed. An explicit terminal
    /// status includes closed handoffs even when include_closed is omitted.
    #[serde(default)]
    pub(crate) status: Option<String>,
    /// Include completed and failed handoffs.
    #[serde(default)]
    pub(crate) include_closed: Option<bool>,
    /// Maximum rows, clamped to 1..100.
    #[serde(default)]
    pub(crate) limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetHandoffParams {
    /// Handoff id returned by send_handoff or list_handoffs.
    pub(crate) id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ClaimHandoffParams {
    /// Handoff id returned by send_handoff or list_handoffs.
    pub(crate) id: String,
    /// Registered agent identity claiming the handoff.
    pub(crate) claimant: String,
    /// Claim token returned by register_agent.
    pub(crate) claim_token: String,
    /// Lease duration in minutes. Defaults to 60; capped at 24 hours.
    #[serde(default)]
    pub(crate) lease_minutes: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CompleteHandoffParams {
    /// Handoff id returned by send_handoff or list_handoffs.
    pub(crate) id: String,
    /// Registered agent identity that claimed the handoff.
    pub(crate) claimant: String,
    /// Claim token returned by register_agent.
    pub(crate) claim_token: String,
    /// completed or failed.
    pub(crate) status: String,
    /// Optional failure or completion note.
    #[serde(default)]
    pub(crate) note: Option<String>,
}
