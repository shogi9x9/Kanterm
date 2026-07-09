use schemars::JsonSchema;
use serde::Deserialize;

use super::nullable_i64_patch;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateParams {
    /// Project board slug/name to target. Required. Existing slugs are used as-is;
    /// unknown values create a workflow-template project board. Cannot target Backlog.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// Card title (required).
    pub(crate) title: String,
    /// Card body / description.
    #[serde(default)]
    pub(crate) body: Option<String>,
    /// Column to create the card in; defaults to the first column.
    #[serde(default)]
    pub(crate) column: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateBacklogCardParams {
    /// Card title (required).
    pub(crate) title: String,
    /// Card body / description.
    #[serde(default)]
    pub(crate) body: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateCardsParams {
    /// Project board slug/name to target. Required. Existing slugs are used as-is;
    /// unknown values create a workflow-template project board. Cannot target Backlog.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// Ordered cards to create.
    pub(crate) cards: Vec<CreateCardItem>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateCardItem {
    /// Optional input-local alias for dependency references.
    #[serde(default)]
    pub(crate) alias: Option<String>,
    /// Card title (required).
    pub(crate) title: String,
    /// Card body / description.
    #[serde(default)]
    pub(crate) body: Option<String>,
    /// Column to create the card in; defaults to the first column.
    #[serde(default)]
    pub(crate) column: Option<String>,
    /// Concrete next step for an agent resuming this card.
    #[serde(default)]
    pub(crate) next_action: Option<String>,
    /// Completion criteria for the work.
    #[serde(default)]
    pub(crate) acceptance_criteria: Option<String>,
    /// Agent suitability/cost weight, 1..5. Pass null to clear.
    #[serde(default, deserialize_with = "nullable_i64_patch")]
    pub(crate) agent_weight: Option<Option<i64>>,
    /// Requested reasoning/runtime level for the agent.
    #[serde(default)]
    pub(crate) agent_effort: Option<String>,
    /// Suggested model/profile for this card.
    #[serde(default)]
    pub(crate) suggested_model: Option<String>,
    /// Expected token budget. Must be positive.
    #[serde(default, deserialize_with = "nullable_i64_patch")]
    pub(crate) expected_tokens: Option<Option<i64>>,
    /// Human intervention gate: none/review/decision/execution.
    #[serde(default)]
    pub(crate) human_intervention: Option<String>,
    /// Upstream aliases or card keys this generated card depends on.
    #[serde(default)]
    pub(crate) depends_on: Option<Vec<String>>,
}
