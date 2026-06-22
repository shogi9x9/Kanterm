use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use std::collections::{HashMap, HashSet};

use crate::Card;

pub(super) fn validate_acyclic(
    tx: &Transaction<'_>,
    board_id: &str,
    downstream_id: &str,
    new_upstream: &[Card],
) -> Result<()> {
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    let mut stmt = tx.prepare(
        "SELECT upstream_card_id, downstream_card_id
           FROM card_dependencies
          WHERE board_id = ?1 AND downstream_card_id <> ?2",
    )?;
    let existing = stmt
        .query_map(params![board_id, downstream_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (upstream, downstream) in existing {
        edges.entry(upstream).or_default().push(downstream);
    }
    for upstream in new_upstream {
        edges
            .entry(upstream.id.clone())
            .or_default()
            .push(downstream_id.to_string());
    }

    let mut nodes = HashSet::new();
    for (upstream, downstreams) in &edges {
        nodes.insert(upstream.clone());
        nodes.extend(downstreams.iter().cloned());
    }
    let mut temporary = HashSet::new();
    let mut permanent = HashSet::new();
    for node in nodes {
        if visit_has_cycle(&node, &edges, &mut temporary, &mut permanent) {
            return Err(anyhow!("card dependency graph would contain a cycle"));
        }
    }
    Ok(())
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
