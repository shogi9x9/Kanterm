use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RegisterAgentParams {
    /// Requested display name, e.g. "codex" or "claude".
    pub(crate) requested_name: String,
    /// Previously assigned identity, e.g. "codex#abc123"; when set, rotates the token.
    #[serde(default)]
    pub(crate) remembered_identity: Option<String>,
    /// Optional caller fingerprint as JSON/text for recovery/debugging.
    #[serde(default)]
    pub(crate) fingerprint: Option<String>,
    /// Registration lifetime in minutes. Defaults to 2 hours; capped at 24 hours.
    #[serde(default)]
    pub(crate) lease_minutes: Option<i64>,
}
