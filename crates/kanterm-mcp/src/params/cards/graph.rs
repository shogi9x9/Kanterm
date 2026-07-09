use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct DependencyGraphParams {
    /// Board slug to target; defaults to the Backlog board.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// When true, hide edges whose upstream or downstream card is already closed.
    #[serde(default)]
    pub(crate) active_only: Option<bool>,
    /// Optional card key. When set, show only that card and its direct
    /// upstream/downstream neighbours.
    #[serde(default)]
    pub(crate) focus: Option<String>,
}
