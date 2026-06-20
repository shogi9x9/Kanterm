use kanban_core::{
    card_is_stale, format_date, now_ms, priority_label, today_start_ms, Card, Store,
};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::lookup::{columns_by_id, resolve_board};
use crate::params::{BoardParam, KeyParams, ListParams};
use crate::render::{
    activity_lines, board_execution_suffix, claim_detail, claim_suffix, due_suffix,
    execution_note_lines, execution_suffix, label_suffix, memory_lines, metadata_i64,
    metadata_value, priority,
};

use super::queue::{classify_queue, dependency_suffix, queue_suffix, QueueMode};

pub(crate) fn get_board(
    store: &Store,
    default_board_id: &str,
    p: BoardParam,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let board = store.board_by_id_or_slug(&board_id).map_err(internal)?;
    let cols = store.columns(&board_id).map_err(internal)?;
    let cards = store.cards(&board_id).map_err(internal)?;
    let labels = store.labels_by_card(&board_id).map_err(internal)?;
    let mut out = String::new();
    if let Some(context) = board.agent_context.as_deref() {
        out.push_str("board_agent_context:\n");
        out.push_str(context);
        out.push_str("\n\n");
    }
    for col in &cols {
        let in_col: Vec<&Card> = cards.iter().filter(|c| c.column_id == col.id).collect();
        out.push_str(&format!("## {} ({})\n", col.name, in_col.len()));
        for c in in_col {
            out.push_str(&format!(
                "- {} {}{}{}{}\n",
                c.key,
                c.title,
                due_suffix(c),
                label_suffix(&labels, &c.id),
                board_execution_suffix(c)
            ));
        }
        out.push('\n');
    }

    let boards = store.list_boards_all().map_err(internal)?;
    let line = |archived: bool| {
        boards
            .iter()
            .filter(|b| b.archived_at.is_some() == archived)
            .map(|b| {
                let context = if b.agent_context.is_some() {
                    " [context]"
                } else {
                    ""
                };
                if b.id == board_id {
                    format!("{}{} (current)", b.slug, context)
                } else {
                    format!("{}{}", b.slug, context)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str("---\nboards: ");
    out.push_str(&line(false));
    let archived = line(true);
    if !archived.is_empty() {
        out.push_str(&format!("\narchived boards: {archived}"));
    }
    Ok(out)
}

pub(crate) fn list_cards(
    store: &Store,
    default_board_id: &str,
    p: ListParams,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let names = columns_by_id(store, &board_id)?;
    let cards = match p.query.as_deref() {
        Some(query) if !query.trim().is_empty() => {
            store.search_cards(&board_id, query).map_err(internal)?
        }
        _ => store.cards(&board_id).map_err(internal)?,
    };
    let labels = store.labels_by_card(&board_id).map_err(internal)?;
    let queue = p.queue.as_deref().map(QueueMode::parse).transpose()?;
    let now = now_ms();
    let ranked = p.ranked.unwrap_or(false);
    let mut entries = Vec::new();
    for c in &cards {
        let col_name = names.get(&c.column_id).cloned().unwrap_or_default();
        if let Some(col) = &p.column {
            if &col_name != col {
                continue;
            }
        }
        let agent_state_filter = p.agent_state.as_ref().or(p.status.as_ref());
        if let Some(st) = agent_state_filter {
            if &c.agent_state != st {
                continue;
            }
        }
        if let Some(stale) = p.stale {
            if card_is_stale(c) != stale {
                continue;
            }
        }
        if let Some(max) = p.agent_weight_max {
            if c.agent_weight.is_none_or(|weight| weight > max) {
                continue;
            }
        }
        if let Some(agent_effort) = p.agent_effort.as_deref() {
            if c.agent_effort.as_deref() != Some(agent_effort) {
                continue;
            }
        }
        if let Some(suggested_model) = p.suggested_model.as_deref() {
            if c.suggested_model.as_deref() != Some(suggested_model) {
                continue;
            }
        }
        if let Some(min) = p.expected_tokens_min {
            if c.expected_tokens.is_none_or(|tokens| tokens < min) {
                continue;
            }
        }
        if let Some(max) = p.expected_tokens_max {
            if c.expected_tokens.is_none_or(|tokens| tokens > max) {
                continue;
            }
        }
        if let Some(human_intervention) = p.human_intervention.as_deref() {
            let current = c.human_intervention.as_deref().unwrap_or("none");
            if current != human_intervention {
                continue;
            }
        }
        let readiness = store.card_readiness(&board_id, &c.key).map_err(internal)?;
        let queue_status = classify_queue(c, now, &readiness);
        if let Some(queue) = queue {
            if !queue.matches(queue_status) {
                continue;
            }
        }
        let stale = if card_is_stale(c) { " [stale]" } else { "" };
        let workflow = if c.blocked_reason.is_some() {
            " [blocked]"
        } else if c.next_action.is_some() {
            " [next]"
        } else {
            ""
        };
        let claim = claim_suffix(c);
        let rank = rank_key(c);
        let rank_reason = if ranked {
            rank_reason(c)
        } else {
            String::new()
        };
        entries.push((
            rank,
            c.key.clone(),
            format!(
                "{}  [{}] ({}) {}{}{}{}{}{}{}{}{}{}",
                c.key,
                col_name,
                priority(c),
                c.title,
                due_suffix(c),
                label_suffix(&labels, &c.id),
                workflow,
                claim,
                stale,
                execution_suffix(c),
                dependency_suffix(queue_status, &readiness),
                queue_suffix(queue, queue_status),
                rank_reason,
            ),
        ));
    }
    if ranked {
        entries.sort_by_key(|(rank, key, _)| (*rank, key.clone()));
    }
    let lines = entries
        .into_iter()
        .map(|(_, _, line)| line)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        Ok("(no matching cards)".into())
    } else {
        Ok(lines.join("\n"))
    }
}

fn rank_key(card: &Card) -> (i64, i64, i64, i64) {
    (
        -card.priority,
        card.agent_weight.unwrap_or(3),
        card.expected_tokens.unwrap_or(i64::MAX),
        card.updated_at,
    )
}

fn rank_reason(card: &Card) -> String {
    let weight = card
        .agent_weight
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".into());
    let tokens = card
        .expected_tokens
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".into());
    format!(
        " [rank:priority={} weight={} tokens={}]",
        priority_label(card.priority),
        weight,
        tokens
    )
}

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
        body = if card.body.is_empty() { "(no description)" } else { &card.body },
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

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_core::CardPatch;

    #[test]
    fn list_cards_query_matches_next_action() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        store
            .create_card(&board.id, None, "ordinary title", "", "test")
            .unwrap();
        store
            .update_card(
                &board.id,
                "KB-1",
                &CardPatch {
                    next_action: Some("Run cargo mutants".to_string()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        let out = list_cards(
            &store,
            &board.id,
            ListParams {
                board: None,
                column: None,
                agent_state: None,
                status: None,
                query: Some("mutants".to_string()),
                stale: None,
                agent_weight_max: None,
                agent_effort: None,
                suggested_model: None,
                expected_tokens_min: None,
                expected_tokens_max: None,
                human_intervention: None,
                queue: None,
                ranked: None,
            },
        )
        .unwrap();

        assert!(out.contains("KB-1"));
        assert!(out.contains("[next]"));

        let fresh = list_cards(
            &store,
            &board.id,
            ListParams {
                board: None,
                column: None,
                agent_state: None,
                status: None,
                query: None,
                stale: Some(false),
                agent_weight_max: None,
                agent_effort: None,
                suggested_model: None,
                expected_tokens_min: None,
                expected_tokens_max: None,
                human_intervention: None,
                queue: None,
                ranked: None,
            },
        )
        .unwrap();
        assert!(fresh.contains("KB-1"));

        let stale = list_cards(
            &store,
            &board.id,
            ListParams {
                board: None,
                column: None,
                agent_state: None,
                status: None,
                query: None,
                stale: Some(true),
                agent_weight_max: None,
                agent_effort: None,
                suggested_model: None,
                expected_tokens_min: None,
                expected_tokens_max: None,
                human_intervention: None,
                queue: None,
                ranked: None,
            },
        )
        .unwrap();
        assert!(stale.contains("no matching"));
    }
}
