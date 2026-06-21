use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use std::collections::{HashMap, HashSet};

use crate::activity::log_activity_payload;
use crate::rows::{row_to_card, CARD_SELECT_BY_KEY};
use crate::{
    now_ms, Card, CardDependency, CardReadiness, DependencyBlockedCard, DependencyBlocker,
    DependencyStagePlan, Store,
};

impl Store {
    pub fn card_dependencies(&self, board_id: &str) -> Result<Vec<CardDependency>> {
        load_dependencies(&self.conn, board_id)
    }

    pub fn card_upstream_dependencies(
        &self,
        board_id: &str,
        key: &str,
    ) -> Result<Vec<CardDependency>> {
        let card = self
            .card_by_key(board_id, key)?
            .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
        load_dependencies_for_downstream(&self.conn, board_id, &card.id)
    }

    pub fn set_card_dependencies(
        &mut self,
        board_id: &str,
        key: &str,
        upstream_keys: &[String],
        actor: &str,
    ) -> Result<Vec<CardDependency>> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let downstream_id = set_card_dependencies_tx(&tx, board_id, key, upstream_keys, actor)?;
        let dependencies = load_dependencies_for_downstream(&tx, board_id, &downstream_id)?;
        tx.commit()?;
        Ok(dependencies)
    }

    pub(crate) fn set_card_dependencies_in_tx(
        tx: &Transaction<'_>,
        board_id: &str,
        key: &str,
        upstream_keys: &[String],
        actor: &str,
    ) -> Result<()> {
        set_card_dependencies_tx(tx, board_id, key, upstream_keys, actor).map(|_| ())
    }

    pub fn card_readiness(&self, board_id: &str, key: &str) -> Result<CardReadiness> {
        let card = self
            .card_by_key(board_id, key)?
            .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
        let upstream = load_upstream_cards_for_card(&self.conn, board_id, &card.id)?;
        Ok(readiness_for_card(&card, upstream))
    }

    pub fn dependency_stage_plan(&self, board_id: &str) -> Result<DependencyStagePlan> {
        let cards = self.cards(board_id)?;
        let dependencies = self.card_dependencies(board_id)?;
        stage_plan_from_cards(self, board_id, &cards, &dependencies)
    }
}

fn set_card_dependencies_tx(
    tx: &Transaction<'_>,
    board_id: &str,
    key: &str,
    upstream_keys: &[String],
    actor: &str,
) -> Result<String> {
    let downstream = load_card_by_key_tx(tx, board_id, key)?
        .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
    let upstream = load_upstream_cards(tx, board_id, &downstream, upstream_keys)?;
    validate_acyclic(tx, board_id, &downstream.id, &upstream)?;

    tx.execute(
        "DELETE FROM card_dependencies WHERE board_id = ?1 AND downstream_card_id = ?2",
        params![board_id, downstream.id],
    )?;
    let ts = now_ms();
    for card in &upstream {
        tx.execute(
            "INSERT INTO card_dependencies
               (board_id, downstream_card_id, upstream_card_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![board_id, downstream.id, card.id, ts],
        )?;
    }
    log_activity_payload(
        tx,
        &downstream.id,
        actor,
        "dependencies",
        serde_json::json!({
            "detail": format!("{} depends on {}", downstream.key, upstream.iter().map(|c| c.key.as_str()).collect::<Vec<_>>().join(", ")),
        }),
    )?;
    Ok(downstream.id)
}

fn load_card_by_key_tx(tx: &Transaction<'_>, board_id: &str, key: &str) -> Result<Option<Card>> {
    tx.query_row(CARD_SELECT_BY_KEY, params![board_id, key], row_to_card)
        .optional()
        .map_err(Into::into)
}

fn load_upstream_cards(
    tx: &Transaction<'_>,
    board_id: &str,
    downstream: &Card,
    upstream_keys: &[String],
) -> Result<Vec<Card>> {
    let mut seen = HashSet::new();
    let mut cards = Vec::new();
    for key in upstream_keys {
        if !seen.insert(key.clone()) {
            continue;
        }
        if key == &downstream.key {
            return Err(anyhow!("card '{}' cannot depend on itself", downstream.key));
        }
        let upstream = load_card_by_key_tx(tx, board_id, key)?
            .ok_or_else(|| anyhow!("dependency references missing card '{key}'"))?;
        cards.push(upstream);
    }
    Ok(cards)
}

fn load_dependencies(conn: &rusqlite::Connection, board_id: &str) -> Result<Vec<CardDependency>> {
    let mut stmt = conn.prepare(
        "SELECT d.board_id,
                d.downstream_card_id,
                down.key_text,
                d.upstream_card_id,
                up.key_text,
                d.created_at
           FROM card_dependencies d
           JOIN cards down ON down.id = d.downstream_card_id
           JOIN cards up ON up.id = d.upstream_card_id
          WHERE d.board_id = ?1
          ORDER BY down.key_text, up.key_text",
    )?;
    collect_dependencies(&mut stmt, params![board_id])
}

fn load_dependencies_for_downstream(
    conn: &rusqlite::Connection,
    board_id: &str,
    downstream_card_id: &str,
) -> Result<Vec<CardDependency>> {
    let mut stmt = conn.prepare(
        "SELECT d.board_id,
                d.downstream_card_id,
                down.key_text,
                d.upstream_card_id,
                up.key_text,
                d.created_at
           FROM card_dependencies d
           JOIN cards down ON down.id = d.downstream_card_id
           JOIN cards up ON up.id = d.upstream_card_id
          WHERE d.board_id = ?1 AND d.downstream_card_id = ?2
          ORDER BY up.key_text",
    )?;
    collect_dependencies(&mut stmt, params![board_id, downstream_card_id])
}

fn collect_dependencies<P>(
    stmt: &mut rusqlite::Statement<'_>,
    params: P,
) -> Result<Vec<CardDependency>>
where
    P: rusqlite::Params,
{
    stmt.query_map(params, |r| {
        Ok(CardDependency {
            board_id: r.get(0)?,
            downstream_card_id: r.get(1)?,
            downstream_key: r.get(2)?,
            upstream_card_id: r.get(3)?,
            upstream_key: r.get(4)?,
            created_at: r.get(5)?,
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

fn load_upstream_cards_for_card(
    conn: &rusqlite::Connection,
    board_id: &str,
    downstream_card_id: &str,
) -> Result<Vec<Card>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.board_id, c.column_id, c.key_text, c.title, c.body, c.status,
                c.priority, c.assignee, c.due_date, c.next_action, c.blocked_reason,
                c.acceptance_criteria, c.handoff_note, c.last_verification, c.agent_weight,
                c.agent_effort, c.suggested_model, c.expected_tokens, c.human_intervention,
                c.claimed_by, c.claimed_at, c.lease_expires_at, c.position, c.created_at,
                c.updated_at, c.archived_at
           FROM card_dependencies d
           JOIN cards c ON c.id = d.upstream_card_id
          WHERE d.board_id = ?1 AND d.downstream_card_id = ?2
          ORDER BY c.key_text",
    )?;
    let cards = stmt
        .query_map(params![board_id, downstream_card_id], row_to_card)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(cards)
}

fn readiness_for_card(card: &Card, upstream: Vec<Card>) -> CardReadiness {
    let blocked_by = upstream
        .into_iter()
        .filter(|c| c.agent_state != "done")
        .map(|c| DependencyBlocker {
            key: c.key,
            title: c.title,
            agent_state: c.agent_state,
            archived_at: c.archived_at,
        })
        .collect::<Vec<_>>();
    let closed = card.is_closed();
    CardReadiness {
        card_key: card.key.clone(),
        ready: !closed && blocked_by.is_empty(),
        closed,
        blocked_by,
    }
}

fn stage_plan_from_cards(
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

fn validate_acyclic(
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
