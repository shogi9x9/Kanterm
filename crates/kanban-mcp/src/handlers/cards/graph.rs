use kanban_core::{now_ms, Card, HumanIntervention, Store};
use rmcp::ErrorData;
use std::collections::HashMap;

use crate::error::{bad_param, internal};
use crate::lookup::resolve_board;
use crate::params::DependencyGraphParams;

pub(crate) fn dependency_graph(
    store: &Store,
    default_board_id: &str,
    p: DependencyGraphParams,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let cards = store.cards(&board_id).map_err(internal)?;
    let card_by_key = cards
        .iter()
        .map(|card| (card.key.as_str(), card))
        .collect::<HashMap<_, _>>();
    let focus = p.focus.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(key) = focus {
        if !card_by_key.contains_key(key) {
            return Err(bad_param(format!("no card '{key}'")));
        }
    }
    let dependencies = filter_dependency_graph_edges(
        store.card_dependencies(&board_id).map_err(internal)?,
        &card_by_key,
        p.active_only.unwrap_or(false),
        focus,
    );
    let stages = store.dependency_stage_plan(&board_id).map_err(internal)?;
    let node_states = graph_node_states(store, &board_id, &stages)?;
    let edges = if dependencies.is_empty() {
        "-".into()
    } else {
        dependencies
            .iter()
            .map(|d| format!("- {} -> {}", d.upstream_key, d.downstream_key))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let stage_lines = if stages.ready_stages.is_empty() {
        "-".into()
    } else {
        stages
            .ready_stages
            .iter()
            .enumerate()
            .map(|(idx, keys)| {
                let nodes = keys
                    .iter()
                    .map(|key| graph_node(key, &node_states))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("stage {}: {}", idx + 1, nodes)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let blocked = if stages.dependency_blocked.is_empty() {
        "-".into()
    } else {
        stages
            .dependency_blocked
            .iter()
            .map(|blocked| {
                format!(
                    "- {} blocked_by: {}",
                    graph_node(&blocked.key, &node_states),
                    blocked
                        .blocked_by
                        .iter()
                        .map(|key| graph_node(key, &node_states))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(format!(
        "dependency_graph:\nedges:\n{edges}\nstages:\n{stage_lines}\ndependency_blocked:\n{blocked}"
    ))
}

fn graph_node_states(
    store: &Store,
    board_id: &str,
    stages: &kanban_core::DependencyStagePlan,
) -> Result<HashMap<String, String>, ErrorData> {
    let mut states = HashMap::new();
    for key in stages
        .ready_stages
        .iter()
        .flatten()
        .chain(stages.dependency_blocked.iter().map(|blocked| &blocked.key))
    {
        let Some(card) = store.card_by_key(board_id, key).map_err(internal)? else {
            continue;
        };
        let readiness = store.card_readiness(board_id, key).map_err(internal)?;
        states.insert(key.clone(), graph_node_state(&card, &readiness));
    }
    Ok(states)
}

fn graph_node_state(card: &Card, readiness: &kanban_core::CardReadiness) -> String {
    if card.agent_state == "done" || card.archived_at.is_some() {
        return "done".into();
    }
    if claim_is_active_for_graph(card) {
        return "running".into();
    }
    match card.human_gate() {
        Some(HumanIntervention::Review) => "human:review".into(),
        Some(HumanIntervention::Decision) => "human:decision".into(),
        Some(HumanIntervention::Execution) => "human:execution".into(),
        None if !readiness.ready => "dep-blocked".into(),
        None if card.blocked_reason.is_some() => "blocked".into(),
        None if card.next_action.is_none() || card.acceptance_criteria.is_none() => {
            "missing".into()
        }
        None => "ready".into(),
    }
}

fn claim_is_active_for_graph(card: &Card) -> bool {
    card.claimed_by.is_some()
        && card
            .lease_expires_at
            .is_some_and(|expires_at| expires_at > now_ms())
}

fn graph_node(key: &str, states: &HashMap<String, String>) -> String {
    states
        .get(key)
        .map(|state| format!("{key}({state})"))
        .unwrap_or_else(|| key.to_string())
}

fn filter_dependency_graph_edges(
    dependencies: Vec<kanban_core::CardDependency>,
    card_by_key: &HashMap<&str, &Card>,
    active_only: bool,
    focus: Option<&str>,
) -> Vec<kanban_core::CardDependency> {
    dependencies
        .into_iter()
        .filter(|dep| {
            if active_only {
                let Some(upstream) = card_by_key.get(dep.upstream_key.as_str()) else {
                    return false;
                };
                let Some(downstream) = card_by_key.get(dep.downstream_key.as_str()) else {
                    return false;
                };
                if is_closed(upstream) || is_closed(downstream) {
                    return false;
                }
            }
            if let Some(focus) = focus {
                dep.upstream_key == focus || dep.downstream_key == focus
            } else {
                true
            }
        })
        .collect()
}

fn is_closed(card: &Card) -> bool {
    card.agent_state == "done" || card.archived_at.is_some()
}
