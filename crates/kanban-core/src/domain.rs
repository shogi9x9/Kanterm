use serde::{Deserialize, Serialize};

use crate::now_ms;

pub const STALE_CARD_MS: i64 = 30 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub key_prefix: String,
    pub card_seq: i64,
    pub sort_order: i64,
    pub archived_at: Option<i64>,
    pub agent_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub sort_order: i64,
    pub wip_limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub board_id: String,
    pub column_id: String,
    pub key: String,
    pub title: String,
    pub body: String,
    pub agent_state: String,
    pub priority: i64,
    pub assignee: Option<String>,
    pub due_date: Option<i64>,
    pub next_action: Option<String>,
    pub blocked_reason: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub handoff_note: Option<String>,
    pub last_verification: Option<String>,
    pub agent_weight: Option<i64>,
    pub agent_effort: Option<String>,
    pub suggested_model: Option<String>,
    pub expected_tokens: Option<i64>,
    pub human_intervention: Option<String>,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<i64>,
    pub lease_expires_at: Option<i64>,
    pub position: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardDependency {
    pub board_id: String,
    pub downstream_card_id: String,
    pub downstream_key: String,
    pub upstream_card_id: String,
    pub upstream_key: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyBlocker {
    pub key: String,
    pub title: String,
    pub agent_state: String,
    pub archived_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardReadiness {
    pub card_key: String,
    pub ready: bool,
    pub closed: bool,
    pub blocked_by: Vec<DependencyBlocker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyBlockedCard {
    pub key: String,
    pub blocked_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyStagePlan {
    pub ready_stages: Vec<Vec<String>>,
    pub dependency_blocked: Vec<DependencyBlockedCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub actor: String,
    pub action: String,
    pub payload_json: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub id: String,
    pub requested_name: String,
    pub assigned_identity: String,
    pub fingerprint_json: Option<String>,
    pub registered_at: i64,
    pub last_seen_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResult {
    pub registration: AgentRegistration,
    /// Secret returned once to the caller. The database stores only a hash.
    pub claim_token: String,
}

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

/// A remembered decision/learning/context note, addressed by a global key
/// like `M-12`. Linked to cards only loosely (by key text) so it survives
/// board archive/delete - memories are the long-lived layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub key: String,
    pub title: String,
    pub body: String,
    pub kind: String,
    pub card_key: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
    pub last_recalled_at: Option<i64>,
    pub recall_count: i64,
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

pub const PRIORITY_LOW: i64 = 0;
pub const PRIORITY_NORMAL: i64 = 1;
pub const PRIORITY_HIGH: i64 = 2;

pub fn priority_label(p: i64) -> &'static str {
    match p {
        PRIORITY_LOW => "low",
        PRIORITY_HIGH => "high",
        _ => "normal",
    }
}

pub fn priority_badge(p: i64) -> &'static str {
    match p {
        PRIORITY_LOW => "[L]",
        PRIORITY_HIGH => "[H]",
        _ => "[M]",
    }
}

pub fn card_is_stale(card: &Card) -> bool {
    now_ms().saturating_sub(card.updated_at) > STALE_CARD_MS
}
