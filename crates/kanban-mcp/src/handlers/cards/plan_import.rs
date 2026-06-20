use kanban_core::{HumanIntervention, Store};
use rmcp::ErrorData;
use std::collections::{HashMap, HashSet};

use crate::error::{bad_param, internal};
use crate::params::CreateCardItem;

pub(super) fn preflight_create_cards(
    store: &Store,
    board_id: &str,
    cards: &[CreateCardItem],
) -> Result<(), ErrorData> {
    let columns = store.columns(board_id).map_err(internal)?;
    let column_names = columns
        .iter()
        .map(|c| c.name.as_str())
        .collect::<HashSet<_>>();
    let mut aliases = HashSet::new();
    for (idx, item) in cards.iter().enumerate() {
        if item.title.trim().is_empty() {
            return Err(bad_param(format!("cards[{idx}].title must not be empty")));
        }
        if let Some(column) = item.column.as_deref() {
            if !column_names.contains(column) {
                return Err(bad_param(format!("no column named '{column}'")));
            }
        }
        if let Some(alias) = item.alias.as_deref() {
            if alias.trim().is_empty() {
                return Err(bad_param(format!("cards[{idx}].alias must not be empty")));
            }
            if !aliases.insert(alias.to_string()) {
                return Err(bad_param(format!("duplicate card alias '{alias}'")));
            }
            if store
                .card_by_key(board_id, alias)
                .map_err(internal)?
                .is_some()
            {
                return Err(bad_param(format!(
                    "cards[{idx}].alias conflicts with existing card key '{alias}'"
                )));
            }
        }
        if let Some(Some(weight)) = item.agent_weight {
            if !(1..=5).contains(&weight) {
                return Err(bad_param("agent_weight must be between 1 and 5"));
            }
        }
        if let Some(Some(tokens)) = item.expected_tokens {
            if tokens <= 0 {
                return Err(bad_param("expected_tokens must be positive"));
            }
        }
        if let Some(human) = item.human_intervention.as_deref() {
            HumanIntervention::parse(human).map_err(bad_param)?;
        }
    }

    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for dep in store.card_dependencies(board_id).map_err(internal)? {
        edges
            .entry(dep.upstream_key)
            .or_default()
            .push(dep.downstream_key);
    }
    for (idx, item) in cards.iter().enumerate() {
        let downstream = planned_node_name(idx, item);
        for dependency in item.depends_on.clone().unwrap_or_default() {
            let dependency_exists = aliases.contains(&dependency)
                || store
                    .card_by_key(board_id, &dependency)
                    .map_err(internal)?
                    .is_some();
            let upstream = if dependency_exists {
                dependency
            } else {
                return Err(bad_param(format!(
                    "cards[{idx}].depends_on references unknown alias or key '{dependency}'"
                )));
            };
            edges.entry(upstream).or_default().push(downstream.clone());
        }
    }
    if graph_has_cycle(&edges) {
        return Err(bad_param("card dependency graph would contain a cycle"));
    }
    Ok(())
}

fn planned_node_name(idx: usize, item: &CreateCardItem) -> String {
    item.alias
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| format!("$new:{idx}"))
}

fn graph_has_cycle(edges: &HashMap<String, Vec<String>>) -> bool {
    let mut nodes = HashSet::new();
    for (upstream, downstreams) in edges {
        nodes.insert(upstream.clone());
        nodes.extend(downstreams.iter().cloned());
    }
    let mut temporary = HashSet::new();
    let mut permanent = HashSet::new();
    nodes
        .iter()
        .any(|node| visit_has_cycle(node, edges, &mut temporary, &mut permanent))
}

fn visit_has_cycle(
    node: &str,
    edges: &HashMap<String, Vec<String>>,
    temporary: &mut HashSet<String>,
    permanent: &mut HashSet<String>,
) -> bool {
    if permanent.contains(node) {
        return false;
    }
    if !temporary.insert(node.to_string()) {
        return true;
    }
    if let Some(next) = edges.get(node) {
        for child in next {
            if visit_has_cycle(child, edges, temporary, permanent) {
                return true;
            }
        }
    }
    temporary.remove(node);
    permanent.insert(node.to_string());
    false
}
