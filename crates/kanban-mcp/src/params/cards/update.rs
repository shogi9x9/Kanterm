use schemars::JsonSchema;
use serde::Deserialize;

use super::nullable_i64_patch;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct LastVerificationParam {
    /// Command that was run, e.g. "cargo test".
    pub(crate) command: String,
    /// Result status, e.g. "passed", "failed", or "blocked".
    pub(crate) status: String,
    /// Short verification summary.
    pub(crate) summary: String,
    /// Optional epoch-milliseconds timestamp. Defaults to now.
    #[serde(default)]
    pub(crate) timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UpdateParams {
    /// Board slug to target; defaults to the Backlog board.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// The card key to update, e.g. "KB-12".
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) body: Option<String>,
    #[serde(default)]
    pub(crate) agent_state: Option<String>,
    /// Deprecated alias for agent_state.
    #[serde(default)]
    pub(crate) status: Option<String>,
    /// Priority: 0 = low, 1 = normal, 2 = high.
    #[serde(default)]
    pub(crate) priority: Option<i64>,
    #[serde(default)]
    pub(crate) assignee: Option<String>,
    /// Concrete next step for an agent resuming this card. Pass "" to clear.
    #[serde(default)]
    pub(crate) next_action: Option<String>,
    /// Why this card is blocked. Pass "" to clear.
    #[serde(default)]
    pub(crate) blocked_reason: Option<String>,
    /// Completion criteria for the work. Pass "" to clear.
    #[serde(default)]
    pub(crate) acceptance_criteria: Option<String>,
    /// Handoff note for interruption/resumption. Pass "" to clear.
    #[serde(default)]
    pub(crate) handoff_note: Option<String>,
    /// Last verification result for this card.
    #[serde(default)]
    pub(crate) last_verification: Option<LastVerificationParam>,
    /// Append-only execution/resume note. Captures what was tried or what remains.
    #[serde(default)]
    pub(crate) execution_note: Option<String>,
    /// Agent suitability/cost weight, 1..5. Pass null to clear.
    #[serde(default, deserialize_with = "nullable_i64_patch")]
    pub(crate) agent_weight: Option<Option<i64>>,
    /// Requested reasoning/runtime level for the agent. Pass "" to clear.
    #[serde(default)]
    pub(crate) agent_effort: Option<String>,
    /// Suggested model/profile for this card. Pass "" to clear.
    #[serde(default)]
    pub(crate) suggested_model: Option<String>,
    /// Expected token budget. Must be positive. Pass null to clear.
    #[serde(default, deserialize_with = "nullable_i64_patch")]
    pub(crate) expected_tokens: Option<Option<i64>>,
    /// Human intervention gate: none/review/decision/execution. Pass "" to clear.
    #[serde(default)]
    pub(crate) human_intervention: Option<String>,
    /// Upstream card keys this card depends on. Replaces the existing dependency list.
    #[serde(default)]
    pub(crate) depends_on: Option<Vec<String>>,
    /// Claim this card for an agent/user. Pass "" to clear.
    #[serde(default)]
    pub(crate) claim: Option<String>,
    /// Claim token returned by register_agent. Required when claim/release_claim is set.
    #[serde(default)]
    pub(crate) claim_token: Option<String>,
    /// Release any active claim.
    #[serde(default)]
    pub(crate) release_claim: Option<bool>,
    /// Lease duration in minutes for a new/renewed claim. Defaults to 60.
    #[serde(default)]
    pub(crate) lease_minutes: Option<i64>,
    /// Move the card to this column (e.g. "Doing").
    #[serde(default)]
    pub(crate) column: Option<String>,
    /// Move the card to this board (board slug), instead of updating the current one.
    #[serde(default)]
    pub(crate) move_to_board: Option<String>,
    /// Set true to archive (hide) the card.
    #[serde(default)]
    pub(crate) archived: Option<bool>,
    /// Label names to attach (created on demand).
    #[serde(default)]
    pub(crate) add_labels: Option<Vec<String>>,
    /// Label names to detach.
    #[serde(default)]
    pub(crate) remove_labels: Option<Vec<String>>,
    /// Due date as "YYYY-MM-DD"; pass "" to clear it.
    #[serde(default)]
    pub(crate) due: Option<String>,
    /// Optional optimistic-concurrency guard. If provided, `update_card` succeeds
    /// only if the card's current `updated_at` still matches this value.
    #[serde(default)]
    pub(crate) expected_updated_at: Option<i64>,
    /// Optional note added when completing a card. Appended as
    /// "[completion note] ..." and archived=true.
    #[serde(default)]
    pub(crate) complete_note: Option<String>,
}
