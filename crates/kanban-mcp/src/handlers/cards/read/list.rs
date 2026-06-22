use kanban_core::{card_is_stale, now_ms, priority_label, Card, Store};
use rmcp::ErrorData;

use crate::error::internal;
use crate::lookup::{columns_by_id, resolve_board};
use crate::params::ListParams;
use crate::render::{claim_suffix, due_suffix, execution_suffix, label_suffix, priority};

use super::super::queue::{classify_queue, dependency_suffix, queue_suffix, QueueMode};

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
