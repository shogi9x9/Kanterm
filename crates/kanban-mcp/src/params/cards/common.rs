use schemars::JsonSchema;
use serde::Deserialize;

/// Empty-but-addressable: lets `get_board` optionally target a board by slug.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct BoardParam {
    /// Board slug to target; defaults to the Backlog board. See the board list at
    /// the bottom of `get_board`.
    #[serde(default)]
    pub(crate) board: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KeyParams {
    /// Board slug to target; defaults to the Backlog board.
    #[serde(default)]
    pub(crate) board: Option<String>,
    /// The card key, e.g. "KB-12".
    pub(crate) key: String,
}
