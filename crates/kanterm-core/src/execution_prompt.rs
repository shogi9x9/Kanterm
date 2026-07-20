use anyhow::{anyhow, bail, Result};

use crate::{
    AgentWorkPacket, Board, Card, CardReadiness, Column, Label, Store, AGENT_WORK_PACKET_VERSION,
};

pub const EXECUTION_PROMPT_VERSION: &str = AGENT_WORK_PACKET_VERSION;
pub const MAX_EXECUTION_PROMPT_BYTES: usize = 100_000;

#[derive(Debug, Clone)]
pub struct ExecutionPromptSnapshot {
    pub board: Board,
    pub column: Column,
    pub card: Card,
    pub labels: Vec<Label>,
    pub readiness: CardReadiness,
    pub upstream: Vec<Card>,
}

impl Store {
    /// Collect the durable, board-scoped work packet used by execution-prompt
    /// adapters. Clipboard and terminal behavior deliberately stay outside core.
    pub fn execution_prompt_snapshot(
        &self,
        board_id: &str,
        key: &str,
    ) -> Result<ExecutionPromptSnapshot> {
        let board = self.board_by_id_or_slug(board_id)?;
        let resolved_board_id = board.id.clone();
        let card = self
            .card_by_key(&resolved_board_id, key)?
            .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
        let column = self
            .columns(&resolved_board_id)?
            .into_iter()
            .find(|column| column.id == card.column_id)
            .ok_or_else(|| anyhow!("card '{key}' references a missing column"))?;
        let labels = self
            .labels_by_card(&resolved_board_id)?
            .remove(&card.id)
            .unwrap_or_default();
        let readiness = self.card_readiness(&resolved_board_id, key)?;
        let upstream = self.card_upstream_cards(&resolved_board_id, key)?;

        Ok(ExecutionPromptSnapshot {
            board,
            column,
            card,
            labels,
            readiness,
            upstream,
        })
    }

    pub fn execution_prompt(&self, board_id: &str, key: &str) -> Result<String> {
        build_execution_prompt(&self.execution_prompt_snapshot(board_id, key)?)
    }
}

pub fn build_execution_prompt(snapshot: &ExecutionPromptSnapshot) -> Result<String> {
    AgentWorkPacket::execute(snapshot).render()
}

pub fn build_verification_prompt(snapshot: &ExecutionPromptSnapshot) -> Result<String> {
    AgentWorkPacket::verify(snapshot).render()
}

pub fn build_resume_prompt(snapshot: &ExecutionPromptSnapshot) -> Result<String> {
    AgentWorkPacket::resume(snapshot).render()
}

pub(crate) fn append_card_context(out: &mut String, snapshot: &ExecutionPromptSnapshot) {
    line(out, "");
    line(out, "## Board");
    field(out, "name", &snapshot.board.name);
    field(out, "slug", &snapshot.board.slug);
    delimited(
        out,
        "board_context",
        snapshot.board.agent_context.as_deref(),
    );

    line(out, "");
    line(out, "## Selected card");
    field(out, "key", &snapshot.card.key);
    field(out, "title", &snapshot.card.title);
    field(out, "column", &snapshot.column.name);
    field(out, "agent_state", &snapshot.card.agent_state);
    field(out, "priority", &snapshot.card.priority.to_string());
    optional_field(out, "assignee", snapshot.card.assignee.as_deref());
    if let Some(due) = snapshot.card.due_date {
        field(out, "due", &crate::format_date(due));
    }
    if !snapshot.labels.is_empty() {
        field(
            out,
            "labels",
            &snapshot
                .labels
                .iter()
                .map(|label| label.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    delimited(out, "description", Some(&snapshot.card.body));
    delimited(out, "next_action", snapshot.card.next_action.as_deref());
    delimited(
        out,
        "acceptance_criteria",
        snapshot.card.acceptance_criteria.as_deref(),
    );
    delimited(
        out,
        "blocked_reason",
        snapshot.card.blocked_reason.as_deref(),
    );
    optional_field(
        out,
        "human_intervention",
        snapshot
            .card
            .human_intervention
            .as_deref()
            .filter(|value| *value != "none"),
    );
    if let Some(weight) = snapshot.card.agent_weight {
        field(out, "agent_weight", &weight.to_string());
    }
    optional_field(out, "agent_effort", snapshot.card.agent_effort.as_deref());
    optional_field(
        out,
        "suggested_profile",
        snapshot.card.suggested_model.as_deref(),
    );
    if let Some(tokens) = snapshot.card.expected_tokens {
        field(out, "expected_tokens", &tokens.to_string());
    }
    optional_field(out, "claimed_by", snapshot.card.claimed_by.as_deref());
    delimited(out, "handoff_note", snapshot.card.handoff_note.as_deref());
    delimited(
        out,
        "last_verification",
        snapshot.card.last_verification.as_deref(),
    );

    line(out, "");
    line(out, "## Readiness and dependencies");
    let readiness = if snapshot.readiness.closed {
        "closed"
    } else if snapshot.readiness.ready {
        "ready"
    } else {
        "dependency-blocked"
    };
    field(out, "readiness", readiness);
    if snapshot.upstream.is_empty() {
        line(out, "upstream_dependencies: none");
    } else {
        line(out, "upstream_dependencies:");
        for upstream in &snapshot.upstream {
            let state = if upstream.agent_state == "done" {
                "done"
            } else if upstream.archived_at.is_some() {
                "archived-incomplete"
            } else {
                upstream.agent_state.as_str()
            };
            line(
                out,
                &format!(
                    "- {} [{}] {}",
                    upstream.key,
                    state,
                    one_line(&upstream.title)
                ),
            );
        }
    }
}

pub(crate) fn enforce_size(out: &str, subject: &str) -> Result<()> {
    if out.len() > MAX_EXECUTION_PROMPT_BYTES {
        bail!(
            "{subject} is {} bytes; maximum is {} bytes",
            out.len(),
            MAX_EXECUTION_PROMPT_BYTES
        );
    }
    Ok(())
}

pub(crate) fn line(out: &mut String, value: &str) {
    out.push_str(value);
    out.push('\n');
}

pub(crate) fn field(out: &mut String, name: &str, value: &str) {
    line(out, &format!("{name}: {}", one_line(value)));
}

fn optional_field(out: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = non_empty(value) {
        field(out, name, value);
    }
}

pub(crate) fn delimited(out: &mut String, name: &str, value: Option<&str>) {
    let Some(value) = non_empty(value) else {
        return;
    };
    let marker = unique_marker(name, value);
    line(out, &format!("{name}: <<'{marker}'"));
    line(out, value);
    line(out, &marker);
}

fn unique_marker(name: &str, value: &str) -> String {
    let base = format!("KANTERM_{}", name.to_ascii_uppercase());
    let mut marker = base.clone();
    let mut suffix = 0;
    while value.lines().any(|line| line.trim() == marker) {
        suffix += 1;
        marker = format!("{base}_{suffix}");
    }
    marker
}

pub(crate) fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoardColumnTemplate, CardPatch};

    #[test]
    fn prompt_contains_versioned_bounded_work_packet() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Prompt Work", BoardColumnTemplate::Workflow)
            .unwrap();
        store
            .update_board_agent_context(&board.id, Some("Run the workspace checks."))
            .unwrap();
        let upstream = store
            .create_card(&board.id, None, "Prepare fixture", "setup", "test")
            .unwrap();
        let card = store
            .create_card(
                &board.id,
                None,
                "Export selected card",
                "Implement copy.\nDo not hide failures.",
                "test",
            )
            .unwrap();
        store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    next_action: Some("Add a pure formatter.".into()),
                    acceptance_criteria: Some("Golden test passes.".into()),
                    ..CardPatch::default()
                },
                "test",
            )
            .unwrap();
        store
            .set_card_dependencies(
                &board.id,
                &card.key,
                std::slice::from_ref(&upstream.key),
                "test",
            )
            .unwrap();

        let prompt = store.execution_prompt(&board.id, &card.key).unwrap();

        assert!(prompt.starts_with("kanterm-agent-work-packet/v1\nprofile: execute\n"));
        assert!(prompt.contains("slug: prompt-work"));
        assert!(prompt.contains("description: <<'KANTERM_DESCRIPTION'"));
        assert!(prompt.contains("Implement copy.\nDo not hide failures."));
        assert!(prompt.contains("next_action: <<'KANTERM_NEXT_ACTION'"));
        assert!(prompt.contains("readiness: dependency-blocked"));
        assert!(prompt.contains(&format!(
            "- {} [{}] Prepare fixture",
            upstream.key, upstream.agent_state
        )));
        assert!(!prompt.contains(&board.id));
    }

    #[test]
    fn prompt_uses_a_collision_free_deterministic_delimiter() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();
        let card = store
            .create_card(
                &board.id,
                None,
                "Delimiter",
                "first\nKANTERM_DESCRIPTION\nKANTERM_DESCRIPTION_1\nlast",
                "test",
            )
            .unwrap();

        let prompt = store.execution_prompt(&board.id, &card.key).unwrap();

        assert!(prompt.contains("description: <<'KANTERM_DESCRIPTION_2'"));
        assert!(prompt.contains("last\nKANTERM_DESCRIPTION_2\n"));
        assert_eq!(
            prompt,
            store.execution_prompt(&board.id, &card.key).unwrap()
        );
    }
}
