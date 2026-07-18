use anyhow::{anyhow, Result};

use crate::execution_prompt::{delimited, field, line, non_empty, one_line};
use crate::{
    classify_work, now_ms, AgentWorkPacket, Board, Card, CardDependency, CardReadiness, Column,
    DependencyStagePlan, Store, WorkState, AGENT_WORK_PACKET_VERSION,
};

pub const BOARD_EXECUTION_PROMPT_VERSION: &str = AGENT_WORK_PACKET_VERSION;

#[derive(Debug, Clone)]
pub struct BoardCardProgress {
    pub card: Card,
    pub column: Column,
    pub readiness: CardReadiness,
}

#[derive(Debug, Clone)]
pub struct BoardExecutionPromptSnapshot {
    pub board: Board,
    pub cards: Vec<BoardCardProgress>,
    pub dependencies: Vec<CardDependency>,
    pub stage_plan: DependencyStagePlan,
    pub evaluated_at: i64,
}

impl Store {
    pub fn board_execution_prompt_snapshot(
        &self,
        board_id: &str,
    ) -> Result<BoardExecutionPromptSnapshot> {
        let board = self.board_by_id_or_slug(board_id)?;
        let resolved_board_id = board.id.clone();
        let columns = self.columns(&resolved_board_id)?;
        let cards = self
            .cards_including_archived(&resolved_board_id)?
            .into_iter()
            .map(|card| {
                let column = columns
                    .iter()
                    .find(|column| column.id == card.column_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("card '{}' references a missing column", card.key))?;
                let readiness = self.card_readiness(&resolved_board_id, &card.key)?;
                Ok(BoardCardProgress {
                    card,
                    column,
                    readiness,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(BoardExecutionPromptSnapshot {
            board,
            cards,
            dependencies: self.card_dependencies(&resolved_board_id)?,
            stage_plan: self.dependency_stage_plan(&resolved_board_id)?,
            evaluated_at: now_ms(),
        })
    }

    pub fn board_execution_prompt(&self, board_id: &str) -> Result<String> {
        build_board_execution_prompt(&self.board_execution_prompt_snapshot(board_id)?)
    }
}

pub fn build_board_execution_prompt(snapshot: &BoardExecutionPromptSnapshot) -> Result<String> {
    AgentWorkPacket::orient(snapshot).render()
}

pub(crate) fn append_board_context(out: &mut String, snapshot: &BoardExecutionPromptSnapshot) {
    line(out, "");
    line(out, "## Board purpose");
    field(out, "name", &snapshot.board.name);
    field(out, "slug", &snapshot.board.slug);
    delimited(
        out,
        "board_context",
        snapshot.board.agent_context.as_deref(),
    );
    if snapshot
        .board
        .agent_context
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        line(out, "board_context: not set");
    }

    write_progress_summary(out, snapshot);
    write_dependency_stages(out, snapshot);
    write_dependency_graph(out, snapshot);
    write_card_progress(out, snapshot);
}

fn write_progress_summary(out: &mut String, snapshot: &BoardExecutionPromptSnapshot) {
    let mut completed = 0;
    let mut archived_incomplete = 0;
    let mut active = 0;
    let mut ready = 0;
    let mut running = 0;
    let mut human = 0;
    let mut blocked = 0;
    let mut waiting = 0;
    let mut missing = 0;
    for progress in &snapshot.cards {
        if progress.card.agent_state == "done" {
            completed += 1;
            continue;
        }
        if progress.card.archived_at.is_some() {
            archived_incomplete += 1;
            continue;
        }
        active += 1;
        match classify_work(&progress.card, &progress.readiness, snapshot.evaluated_at) {
            WorkState::Executable => ready += 1,
            WorkState::Claimed => running += 1,
            WorkState::Human(_) => human += 1,
            WorkState::Blocked => blocked += 1,
            WorkState::DependencyBlocked => waiting += 1,
            WorkState::MissingContext => missing += 1,
            WorkState::Closed => {}
        }
    }

    line(out, "");
    line(out, "## Progress summary");
    field(out, "total_cards", &snapshot.cards.len().to_string());
    field(out, "completed", &completed.to_string());
    field(out, "active", &active.to_string());
    field(out, "running", &running.to_string());
    field(out, "ready", &ready.to_string());
    field(out, "human_gated", &human.to_string());
    field(out, "explicitly_blocked", &blocked.to_string());
    field(out, "dependency_waiting", &waiting.to_string());
    field(out, "missing_context", &missing.to_string());
    field(out, "archived_incomplete", &archived_incomplete.to_string());
}

fn write_dependency_stages(out: &mut String, snapshot: &BoardExecutionPromptSnapshot) {
    line(out, "");
    line(out, "## Active dependency stages");
    if snapshot.stage_plan.ready_stages.is_empty() {
        line(out, "stages: none");
    } else {
        for (index, keys) in snapshot.stage_plan.ready_stages.iter().enumerate() {
            line(out, &format!("stage {}: {}", index + 1, keys.join(", ")));
        }
    }
    if !snapshot.stage_plan.dependency_blocked.is_empty() {
        line(out, "unresolved_dependency_waiting:");
        for blocked in &snapshot.stage_plan.dependency_blocked {
            line(
                out,
                &format!(
                    "- {} waits for {}",
                    blocked.key,
                    blocked.blocked_by.join(", ")
                ),
            );
        }
    }
}

fn write_dependency_graph(out: &mut String, snapshot: &BoardExecutionPromptSnapshot) {
    line(out, "");
    line(out, "## Dependency graph");
    if snapshot.dependencies.is_empty() {
        line(out, "edges: none");
    } else {
        for dependency in &snapshot.dependencies {
            line(
                out,
                &format!(
                    "- {} [{}] -> {} [{}]",
                    dependency.upstream_key,
                    progress_label(snapshot, &dependency.upstream_key),
                    dependency.downstream_key,
                    progress_label(snapshot, &dependency.downstream_key)
                ),
            );
        }
    }
}

fn write_card_progress(out: &mut String, snapshot: &BoardExecutionPromptSnapshot) {
    line(out, "");
    line(out, "## Card progress");
    if snapshot.cards.is_empty() {
        line(out, "cards: none");
        return;
    }
    for progress in &snapshot.cards {
        line(
            out,
            &format!(
                "- {} [{}] ({}) {}",
                progress.card.key,
                progress_label(snapshot, &progress.card.key),
                one_line(&progress.column.name),
                one_line(&progress.card.title)
            ),
        );
        optional_indented(out, "next", progress.card.next_action.as_deref());
        optional_indented(
            out,
            "acceptance",
            progress.card.acceptance_criteria.as_deref(),
        );
        optional_indented(out, "blocked", progress.card.blocked_reason.as_deref());
        optional_indented(out, "handoff", progress.card.handoff_note.as_deref());
        optional_indented(
            out,
            "verification",
            progress.card.last_verification.as_deref(),
        );
        if !progress.readiness.blocked_by.is_empty() {
            line(
                out,
                &format!(
                    "  waits_for: {}",
                    progress
                        .readiness
                        .blocked_by
                        .iter()
                        .map(|blocker| blocker.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    }
}

fn progress_label(snapshot: &BoardExecutionPromptSnapshot, key: &str) -> String {
    let Some(progress) = snapshot
        .cards
        .iter()
        .find(|progress| progress.card.key == key)
    else {
        return "unknown".into();
    };
    if progress.card.agent_state == "done" {
        return "done".into();
    }
    if progress.card.archived_at.is_some() {
        return "archived-incomplete".into();
    }
    match classify_work(&progress.card, &progress.readiness, snapshot.evaluated_at) {
        WorkState::Closed => "closed".into(),
        WorkState::Blocked => "blocked".into(),
        WorkState::Claimed => "running".into(),
        WorkState::DependencyBlocked => "dependency-waiting".into(),
        WorkState::Human(gate) => format!("human:{}", gate.as_str()),
        WorkState::MissingContext => "missing-context".into(),
        WorkState::Executable => "ready".into(),
    }
}

fn optional_indented(out: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = non_empty(value) {
        line(out, &format!("  {name}: {}", one_line(value)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoardColumnTemplate, CardPatch};

    #[test]
    fn board_prompt_explains_purpose_progress_stages_and_dependencies() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Release Plan", BoardColumnTemplate::Workflow)
            .unwrap();
        store
            .update_board_agent_context(
                &board.id,
                Some("Ship a safe release after implementation and verification."),
            )
            .unwrap();
        let foundation = store
            .create_card(&board.id, None, "Build foundation", "", "test")
            .unwrap();
        let verify = store
            .create_card(&board.id, None, "Verify release", "", "test")
            .unwrap();
        store
            .update_card(
                &board.id,
                &foundation.key,
                &CardPatch {
                    agent_state: Some("done".into()),
                    archived: Some(true),
                    ..CardPatch::default()
                },
                "test",
            )
            .unwrap();
        store
            .update_card(
                &board.id,
                &verify.key,
                &CardPatch {
                    next_action: Some("Run the full verification suite.".into()),
                    acceptance_criteria: Some("All release checks pass.".into()),
                    ..CardPatch::default()
                },
                "test",
            )
            .unwrap();
        store
            .set_card_dependencies(
                &board.id,
                &verify.key,
                std::slice::from_ref(&foundation.key),
                "test",
            )
            .unwrap();

        let snapshot = store.board_execution_prompt_snapshot(&board.id).unwrap();
        let prompt = build_board_execution_prompt(&snapshot).unwrap();

        assert!(prompt.starts_with("kanterm-agent-work-packet/v1\nprofile: orient\n"));
        assert!(prompt
            .contains("This packet is context for orientation. It does not authorize starting"));
        assert!(prompt.contains("Ship a safe release after implementation and verification."));
        assert!(prompt.contains("total_cards: 2"));
        assert!(prompt.contains("completed: 1"));
        assert!(prompt.contains("active: 1"));
        assert!(prompt.contains(&format!("stage 1: {}", verify.key)));
        assert!(prompt.contains(&format!(
            "- {} [done] -> {} [ready]",
            foundation.key, verify.key
        )));
        assert!(prompt.contains(&format!("- {} [done]", foundation.key)));
        assert!(prompt.contains("next: Run the full verification suite."));
        assert!(!prompt.contains(&board.id));
        assert_eq!(prompt, build_board_execution_prompt(&snapshot).unwrap());
    }

    #[test]
    fn board_prompt_reports_missing_context_and_no_edges() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "Needs definition", "", "test")
            .unwrap();

        let prompt = store.board_execution_prompt(&board.id).unwrap();

        assert!(prompt.contains("board_context: not set"));
        assert!(prompt.contains("missing_context: 1"));
        assert!(prompt.contains("edges: none"));
        assert!(prompt.contains(&format!("- {} [missing-context]", card.key)));
    }
}
