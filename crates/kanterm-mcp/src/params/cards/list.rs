use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ListParams {
    /// Board slug to target; defaults to the Backlog board.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// Optional column name filter (e.g. "Todo", "Doing", "Done").
    #[serde(default)]
    pub(crate) column: Option<String>,
    /// Optional agent_state filter (e.g. "open", "working", "handoff").
    #[serde(default)]
    pub(crate) agent_state: Option<String>,
    /// Deprecated alias for agent_state.
    #[serde(default)]
    pub(crate) status: Option<String>,
    /// Optional case-insensitive substring matched against title and body.
    #[serde(default)]
    pub(crate) query: Option<String>,
    /// When true, only cards not updated for the stale threshold are returned.
    /// When false, only non-stale cards are returned. Omit to include both.
    #[serde(default)]
    pub(crate) stale: Option<bool>,
    /// Optional maximum agent execution weight.
    #[serde(default)]
    pub(crate) agent_weight_max: Option<i64>,
    /// Optional exact agent effort filter.
    #[serde(default)]
    pub(crate) agent_effort: Option<String>,
    /// Optional exact suggested model filter.
    #[serde(default)]
    pub(crate) suggested_model: Option<String>,
    /// Optional minimum expected token budget.
    #[serde(default)]
    pub(crate) expected_tokens_min: Option<i64>,
    /// Optional maximum expected token budget.
    #[serde(default)]
    pub(crate) expected_tokens_max: Option<i64>,
    /// Optional human intervention filter. "none" also matches unset values.
    #[serde(default)]
    pub(crate) human_intervention: Option<String>,
    /// Optional queue view: executable, review, blocked, claimed, missing_context,
    /// dependency_blocked, or human.
    #[serde(default)]
    pub(crate) queue: Option<String>,
    /// When true, sort matching cards by next-work suitability and include rank reasons.
    #[serde(default)]
    pub(crate) ranked: Option<bool>,
}
