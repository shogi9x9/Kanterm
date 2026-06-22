use anyhow::Result;
use std::collections::{HashMap, HashSet};

use crate::{Card, CardDependency, DependencyBlockedCard, DependencyStagePlan, Store};

pub(super) fn stage_plan_from_cards(
    store: &Store,
    board_id: &str,
    cards: &[Card],
    dependencies: &[CardDependency],
) -> Result<DependencyStagePlan> {
    let active = cards.iter().filter(|c| !c.is_closed()).collect::<Vec<_>>();
    let active_keys = active.iter().map(|c| c.key.clone()).collect::<HashSet<_>>();
    let mut deps_by_downstream: HashMap<String, Vec<String>> = HashMap::new();
    for dep in dependencies {
        deps_by_downstream
            .entry(dep.downstream_key.clone())
            .or_default()
            .push(dep.upstream_key.clone());
    }

    let mut external_blocked = HashMap::new();
    for card in &active {
        let blockers = store
            .card_readiness(board_id, &card.key)?
            .blocked_by
            .into_iter()
            .map(|b| b.key)
            .filter(|key| !active_keys.contains(key))
            .collect::<Vec<_>>();
        if !blockers.is_empty() {
            external_blocked.insert(card.key.clone(), blockers);
        }
    }

    let mut remaining = active_keys;
    let mut ready_stages = Vec::new();
    while !remaining.is_empty() {
        let ready = active
            .iter()
            .filter(|c| remaining.contains(&c.key))
            .filter(|c| !external_blocked.contains_key(&c.key))
            .filter(|c| {
                deps_by_downstream
                    .get(&c.key)
                    .is_none_or(|upstream| !upstream.iter().any(|key| remaining.contains(key)))
            })
            .map(|c| c.key.clone())
            .collect::<Vec<_>>();
        if ready.is_empty() {
            break;
        }
        for key in &ready {
            remaining.remove(key);
        }
        ready_stages.push(ready);
    }

    let mut dependency_blocked = Vec::new();
    for card in active {
        if remaining.contains(&card.key) {
            let blocked_by = external_blocked.get(&card.key).cloned().unwrap_or_else(|| {
                deps_by_downstream
                    .get(&card.key)
                    .map(|upstream| {
                        upstream
                            .iter()
                            .filter(|key| remaining.contains(*key))
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            });
            dependency_blocked.push(DependencyBlockedCard {
                key: card.key.clone(),
                blocked_by,
            });
        }
    }

    Ok(DependencyStagePlan {
        ready_stages,
        dependency_blocked,
    })
}
