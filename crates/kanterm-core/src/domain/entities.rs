use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandoff {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub board_id: Option<String>,
    pub card_key: Option<String>,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<i64>,
    pub lease_expires_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub failed_at: Option<i64>,
    pub result_text: Option<String>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffDraft {
    pub from_agent: String,
    pub to_agent: String,
    pub board_id: Option<String>,
    pub card_key: Option<String>,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy)]
pub struct HandoffListQuery<'a> {
    pub recipient: Option<&'a str>,
    pub sender: Option<&'a str>,
    pub status: Option<&'a str>,
    pub include_closed: bool,
    /// Restrict to pending or expired-claim work that can be safely executed.
    pub claimable_only: bool,
    pub limit: i64,
}

impl Default for HandoffListQuery<'_> {
    fn default() -> Self {
        Self {
            recipient: None,
            sender: None,
            status: None,
            include_closed: false,
            claimable_only: false,
            limit: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffStatusPatch {
    pub status: String,
    pub note: Option<String>,
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
