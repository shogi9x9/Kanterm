use serde::Deserialize;

/// Partial update for a card. Every field is optional; `None` means "leave it".
/// `column` is a column *name*  (e.g. "This week") so agents never see internal ids.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct CardPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub agent_state: Option<String>,
    pub priority: Option<i64>,
    pub assignee: Option<String>,
    pub column: Option<String>,
    /// Move the card to another board by board id (same schema keyspace as boards.id).
    pub move_to_board: Option<String>,
    pub archived: Option<bool>,
    /// Label names to attach (created on demand).
    pub add_labels: Option<Vec<String>>,
    /// Label names to detach.
    pub remove_labels: Option<Vec<String>>,
    /// Due date as "YYYY-MM-DD". An empty string clears the due date.
    pub due: Option<String>,
    /// Concrete next step for an agent resuming this card. An empty string clears it.
    pub next_action: Option<String>,
    /// Why this card is blocked. An empty string clears it.
    pub blocked_reason: Option<String>,
    /// Completion criteria for the work. An empty string clears it.
    pub acceptance_criteria: Option<String>,
    /// Agent handoff note for interruption/resumption. An empty string clears it.
    pub handoff_note: Option<String>,
    /// Last verification result as JSON text. An empty string clears it.
    pub last_verification: Option<String>,
    /// Agent suitability/cost weight on a small 1..5 scale. `Some(None)` clears it.
    pub agent_weight: Option<Option<i64>>,
    /// Requested reasoning/runtime level for the agent. An empty string clears it.
    pub agent_effort: Option<String>,
    /// Suggested model/profile for this card. An empty string clears it.
    pub suggested_model: Option<String>,
    /// Expected token budget. Must be positive. `Some(None)` clears it.
    pub expected_tokens: Option<Option<i64>>,
    /// Human intervention gate: none/review/decision/execution. An empty string clears it.
    pub human_intervention: Option<String>,
    /// Claim this card for an agent/user. An empty string clears it.
    pub claim: Option<String>,
    /// Secret token returned by register_agent. Required for claim/renew/release.
    pub claim_token: Option<String>,
    /// Release any active claim.
    pub release_claim: Option<bool>,
    /// Lease duration for a new/renewed claim. Defaults to 60 minutes.
    pub lease_minutes: Option<i64>,
    /// Optimistic-concurrency anchor. If set, the update requires the card's
    /// current `updated_at` to match this value.
    pub expected_updated_at: Option<i64>,
}

#[derive(Debug, Default, Clone)]
pub struct CardCreateDraft {
    pub alias: Option<String>,
    pub title: String,
    pub body: String,
    pub column: Option<String>,
    pub next_action: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub agent_weight: Option<Option<i64>>,
    pub agent_effort: Option<String>,
    pub suggested_model: Option<String>,
    pub expected_tokens: Option<Option<i64>>,
    pub human_intervention: Option<String>,
    pub depends_on: Vec<String>,
}

/// Partial update for a memory; `None` means "leave it".
#[derive(Debug, Default, Clone, Deserialize)]
pub struct MemoryPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub kind: Option<String>,
    /// Card key to link to; an empty string clears the link.
    pub card_key: Option<String>,
    pub archived: Option<bool>,
}
