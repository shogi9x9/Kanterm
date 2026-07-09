use kanterm_core::{now_ms, CardPatch, Store};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::lookup::resolve_board;
use crate::params::UpdateParams;
use crate::render::complete_note_body;
use crate::workflow;

use super::super::workflow_trigger::workflow_trigger;

pub(crate) fn update_card(
    store: &mut Store,
    default_board_id: &str,
    p: UpdateParams,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let workflow_trigger = workflow_trigger(&p, store, &board_id)?;
    let execution_note = p.execution_note;
    let depends_on = p.depends_on;
    if depends_on.is_some() && p.move_to_board.is_some() {
        return Err(bad_param(
            "depends_on cannot be combined with move_to_board; dependencies are board-local",
        ));
    }
    let move_to_board = p
        .move_to_board
        .as_deref()
        .map(|target| resolve_board(store, default_board_id, Some(target)))
        .transpose()?;
    let mut body = p.body;
    let mut archived = p.archived;
    let last_verification = p.last_verification.map(|v| {
        serde_json::json!({
            "command": v.command,
            "status": v.status,
            "summary": v.summary,
            "timestamp": v.timestamp.unwrap_or_else(now_ms),
        })
        .to_string()
    });

    let complete_requested = p.complete_note.is_some();
    if let Some(complete_note) = p.complete_note {
        archived = Some(true);
        if !complete_note.trim().is_empty() {
            let base_body = match body {
                Some(ref body_text) => body_text.clone(),
                None => {
                    store
                        .card_by_key(&board_id, &p.key)
                        .map_err(internal)?
                        .ok_or_else(|| bad_param(format!("no card '{}'", p.key)))?
                        .body
                }
            };
            body = Some(complete_note_body(&base_body, complete_note.trim()));
        }
    }
    let patch = CardPatch {
        title: p.title,
        body,
        agent_state: if complete_requested {
            Some("done".into())
        } else {
            p.agent_state.or(p.status)
        },
        priority: p.priority,
        assignee: p.assignee,
        next_action: if complete_requested && p.next_action.is_none() {
            Some(String::new())
        } else {
            p.next_action
        },
        blocked_reason: if complete_requested && p.blocked_reason.is_none() {
            Some(String::new())
        } else {
            p.blocked_reason
        },
        acceptance_criteria: p.acceptance_criteria,
        handoff_note: if complete_requested && p.handoff_note.is_none() {
            Some(String::new())
        } else {
            p.handoff_note
        },
        last_verification,
        agent_weight: p.agent_weight,
        agent_effort: p.agent_effort,
        suggested_model: p.suggested_model,
        expected_tokens: p.expected_tokens,
        human_intervention: p.human_intervention,
        claim: p.claim,
        claim_token: p.claim_token,
        release_claim: p.release_claim,
        lease_minutes: p.lease_minutes,
        column: p.column,
        move_to_board,
        archived,
        add_labels: p.add_labels,
        remove_labels: p.remove_labels,
        due: p.due,
        expected_updated_at: p.expected_updated_at,
    };
    let card = store
        .update_card(&board_id, &p.key, &patch, "agent")
        .map_err(|e| {
            if e.to_string().contains("stale update") {
                bad_param(e)
            } else {
                internal(e)
            }
        })?;
    if let Some(note) = execution_note {
        store
            .record_execution_note(&board_id, &card.key, &note, "agent")
            .map_err(internal)?;
    }
    if let Some(depends_on) = depends_on {
        store
            .set_card_dependencies(&board_id, &card.key, &depends_on, "agent")
            .map_err(internal)?;
    }
    let mut response = format!("updated {}", card.key);
    if complete_requested {
        if let Some(args) = workflow_trigger {
            let summary = workflow::run_summary(store, args).map_err(internal)?;
            response.push_str("\nworkflow_triggered:\n");
            response.push_str(&summary);
        }
    }
    Ok(response)
}
