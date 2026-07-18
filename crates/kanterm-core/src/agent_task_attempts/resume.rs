use anyhow::{anyhow, Result};

use crate::{
    AgentWorkPacketAttemptDelta, AgentWorkPacketResumeDelta, Store, MAX_RESUME_DELTA_CHARS,
};

const MAX_RESUME_ATTEMPTS: usize = 3;
const MAX_RESUME_NOTES: usize = 5;
const MAX_RESUME_ENTRY_CHARS: usize = 990;

impl Store {
    pub fn agent_work_packet_resume_delta(
        &self,
        handoff_id: &str,
        board_id: &str,
        card_key: &str,
    ) -> Result<AgentWorkPacketResumeDelta> {
        let board = self.board_by_id_or_slug(board_id)?;
        let card = self
            .card_by_key(&board.id, card_key)?
            .ok_or_else(|| anyhow!("no card '{card_key}'"))?;
        let attempts = self.agent_task_attempts(handoff_id)?;
        let original_packet_sha256 = attempts
            .first()
            .map(|attempt| attempt.packet_sha256.clone());
        let prior_attempts = attempts
            .iter()
            .rev()
            .take(MAX_RESUME_ATTEMPTS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|attempt| AgentWorkPacketAttemptDelta {
                attempt_no: attempt.attempt_no,
                status: attempt.status.clone(),
                packet_sha256: attempt.packet_sha256.clone(),
                result: attempt
                    .error_text
                    .as_deref()
                    .or(attempt.agent_output.as_deref())
                    .map(|value| truncate_chars(value, MAX_RESUME_ENTRY_CHARS)),
            })
            .collect();
        let execution_notes = self
            .card_activity(&card.id, 50)?
            .into_iter()
            .filter(|activity| activity.action == "execution_note")
            .filter_map(|activity| {
                serde_json::from_str::<serde_json::Value>(&activity.payload_json)
                    .ok()?
                    .get("note")?
                    .as_str()
                    .map(|note| truncate_chars(note, MAX_RESUME_ENTRY_CHARS))
            })
            .take(MAX_RESUME_NOTES)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        let delta = AgentWorkPacketResumeDelta {
            original_packet_sha256,
            prior_attempts,
            execution_notes,
        };
        if delta_size(&delta) > MAX_RESUME_DELTA_CHARS {
            return Err(anyhow!("resume delta exceeded bounded context limit"));
        }
        Ok(delta)
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

pub(super) fn delta_size(delta: &AgentWorkPacketResumeDelta) -> usize {
    delta
        .prior_attempts
        .iter()
        .filter_map(|attempt| attempt.result.as_ref())
        .map(|result| result.chars().count())
        .sum::<usize>()
        + delta
            .execution_notes
            .iter()
            .map(|note| note.chars().count())
            .sum::<usize>()
}
