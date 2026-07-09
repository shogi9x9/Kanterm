use kanterm_core::{classify_graph_node, now_ms, Card, Store};
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
    let node_states = graph_node_states(store, &board_id, &stages, now_ms())?;
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
    stages: &kanterm_core::DependencyStagePlan,
    now: i64,
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
        states.insert(
            key.clone(),
            classify_graph_node(&card, &readiness, now).label(),
        );
    }
    Ok(states)
}

fn graph_node(key: &str, states: &HashMap<String, String>) -> String {
    states
        .get(key)
        .map(|state| format!("{key}({state})"))
        .unwrap_or_else(|| key.to_string())
}

fn filter_dependency_graph_edges(
    dependencies: Vec<kanterm_core::CardDependency>,
    card_by_key: &HashMap<&str, &Card>,
    active_only: bool,
    focus: Option<&str>,
) -> Vec<kanterm_core::CardDependency> {
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
                if upstream.is_closed() || downstream.is_closed() {
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
