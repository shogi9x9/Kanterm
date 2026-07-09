use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RecordMemoryParams {
    /// Short headline (required), e.g. "use rusqlite 0.37, not 0.38".
    pub(crate) title: String,
    /// The substance: reasoning, constraints, links, gotchas. Markdown is fine.
    #[serde(default)]
    pub(crate) body: Option<String>,
    /// One of: decision, learning, context, note. Defaults to "note".
    #[serde(default)]
    pub(crate) kind: Option<String>,
    /// Card key to link this memory to, e.g. "KB-12".
    #[serde(default)]
    pub(crate) card: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RecallMemoriesParams {
    /// Case-insensitive substring matched over title, body and linked card key.
    #[serde(default)]
    pub(crate) query: Option<String>,
    /// Only memories linked to this card key, e.g. "KB-12".
    #[serde(default)]
    pub(crate) card: Option<String>,
    /// Only memories of this kind (decision/learning/context/note).
    #[serde(default)]
    pub(crate) kind: Option<String>,
    /// Maximum number of results (default 10).
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    /// Fetch a single memory in full by key (e.g. "M-3"); other filters are
    /// ignored when set.
    #[serde(default)]
    pub(crate) key: Option<String>,
}
