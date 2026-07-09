use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ManageColumnsParams {
    /// Board slug to target; defaults to the Backlog board.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// One of: add, rename, delete, reorder.
    pub(crate) action: String,
    /// New column name (for `add`).
    #[serde(default)]
    pub(crate) name: Option<String>,
    /// Existing column to act on (for rename/delete/reorder).
    #[serde(default)]
    pub(crate) column: Option<String>,
    /// New name (for `rename`).
    #[serde(default)]
    pub(crate) new_name: Option<String>,
    /// Destination column that receives the cards (for `delete`).
    #[serde(default)]
    pub(crate) to: Option<String>,
    /// "left" or "right" (for `reorder`).
    #[serde(default)]
    pub(crate) direction: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ManageBoardsParams {
    /// One of: create, archive, unarchive, delete, reorder, set_context, clear_context.
    pub(crate) action: String,
    /// Display name for the new board (for `create`). A slug and key prefix are
    /// derived automatically.
    #[serde(default)]
    pub(crate) name: Option<String>,
    /// Column template for `create`: planning, workflow, or simple. Defaults to workflow.
    #[serde(default)]
    pub(crate) template: Option<String>,
    /// Board slug to act on (for `archive`/`unarchive`/`delete`/`reorder`).
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// "up" or "down" (for `reorder`).
    #[serde(default)]
    pub(crate) direction: Option<String>,
    /// Board-level instructions for agents (for `set_context`).
    #[serde(default)]
    pub(crate) agent_context: Option<String>,
}
