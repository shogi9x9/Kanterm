mod persistence;
mod resume;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTaskAttempt {
    pub id: String,
    pub handoff_id: String,
    pub attempt_no: i64,
    pub target_name: String,
    pub packet_version: String,
    pub packet_profile: String,
    pub packet_sha256: String,
    pub packet_text: String,
    pub status: String,
    pub agent_output: Option<String>,
    pub error_text: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[cfg(test)]
mod tests;
