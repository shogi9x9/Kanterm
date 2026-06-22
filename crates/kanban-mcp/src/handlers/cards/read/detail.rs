use kanban_core::{format_date, today_start_ms, Card, Store};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::lookup::{columns_by_id, resolve_board};
use crate::params::KeyParams;
use crate::render::{
    activity_lines, claim_detail, execution_note_lines, memory_lines, metadata_i64, metadata_value,
    priority,
};

pub(crate) fn get_card(
    store: &Store,
    default_board_id: &str,
    p: KeyParams,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let board = store.board_by_id_or_slug(&board_id).map_err(internal)?;
    let names = columns_by_id(store, &board_id)?;
    let card = store
        .card_by_key(&board_id, &p.key)
        .map_err(internal)?
        .ok_or_else(|| bad_param(format!("no card '{}'", p.key)))?;
    let col = names.get(&card.column_id).cloned().unwrap_or_default();
    let labels = store.labels_by_card(&board_id).map_err(internal)?;
    let tags = labels
        .get(&card.id)
        .map(|ls| {
            ls.iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "-".into());
    let due = card
        .due_date
        .map(|ms| {
            let od = if ms < today_start_ms() {
                " (overdue)"
            } else {
                ""
            };
            format!("{}{}", format_date(ms), od)
        })
        .unwrap_or_else(|| "-".into());
    let claim = claim_detail(&card);
    let activity = store.card_activity(&card.id, 20).map_err(internal)?;
    let execution_notes = execution_note_lines(&activity);
    let activity = activity_lines(&activity.iter().take(5).cloned().collect::<Vec<_>>());
    let dependencies = dependency_context(store, &board_id, &card)?;
    let memories = store
        .recall_memories(None, Some(&card.key), None, 5, false)
        .map_err(internal)?;
    let memories = memory_lines(&memories);
    let board_agent_context = metadata_value(board.agent_context.as_deref());
    Ok(format!(
        "{key}  {title}\ncolumn: {col}\npriority: {prio}\nassignee: {assignee}\ndue: {due}\nlabels: {labels}\nboard_agent_context: {board_agent_context}\n\nagent_metadata:\nagent_state: {agent_state}\nclaim: {claim}\nagent_weight: {agent_weight}\nagent_effort: {agent_effort}\nsuggested_model: {suggested_model}\nexpected_tokens: {expected_tokens}\nhuman_intervention: {human_intervention}\nnext_action: {next_action}\nblocked_reason: {blocked_reason}\nacceptance_criteria: {acceptance_criteria}\nhandoff_note: {handoff_note}\nlast_verification: {last_verification}\ndependencies:\n{dependencies}\nexecution_notes:\n{execution_notes}\nactivity:\n{activity}\nrelated_memories:\n{memories}\n\nbody:\n{body}",
        key = card.key,
        title = card.title,
        col = col,
        agent_state = card.agent_state,
        prio = priority(&card),
        assignee = card.assignee.as_deref().unwrap_or("-"),
        due = due,
        labels = tags,
        board_agent_context = board_agent_context,
        claim = claim,
        agent_weight = metadata_i64(card.agent_weight),
        agent_effort = metadata_value(card.agent_effort.as_deref()),
        suggested_model = metadata_value(card.suggested_model.as_deref()),
        expected_tokens = metadata_i64(card.expected_tokens),
        human_intervention = metadata_value(card.human_intervention.as_deref()),
        next_action = metadata_value(card.next_action.as_deref()),
        blocked_reason = metadata_value(card.blocked_reason.as_deref()),
        acceptance_criteria = metadata_value(card.acceptance_criteria.as_deref()),
        handoff_note = metadata_value(card.handoff_note.as_deref()),
        last_verification = metadata_value(card.last_verification.as_deref()),
        dependencies = dependencies,
        execution_notes = execution_notes,
        activity = activity,
        memories = memories,
        body = if card.body.is_empty() {
            "(no description)"
        } else {
            &card.body
        },
    ))
}

fn dependency_context(store: &Store, board_id: &str, card: &Card) -> Result<String, ErrorData> {
    let upstream = store
        .card_upstream_dependencies(board_id, &card.key)
        .map_err(internal)?;
    let all = store.card_dependencies(board_id).map_err(internal)?;
    let downstream = all
        .iter()
        .filter(|d| d.upstream_key == card.key)
        .map(|d| d.downstream_key.as_str())
        .collect::<Vec<_>>();
    let readiness = store
        .card_readiness(board_id, &card.key)
        .map_err(internal)?;
    let state = if readiness.closed {
        "closed".to_string()
    } else if readiness.ready {
        "ready".to_string()
    } else {
        format!(
            "dependency_blocked by {}",
            readiness
                .blocked_by
                .iter()
                .map(|b| b.key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Ok(format!(
        "upstream: {}\ndownstream: {}\nreadiness: {state}",
        list_or_dash(
            upstream
                .iter()
                .map(|d| d.upstream_key.as_str())
                .collect::<Vec<_>>()
        ),
        list_or_dash(downstream),
    ))
}

fn list_or_dash(values: Vec<&str>) -> String {
    if values.is_empty() {
        "-".into()
    } else {
        values.join(", ")
    }
}
