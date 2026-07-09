use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use std::collections::{HashMap, HashSet};

use crate::activity::log_activity;
use crate::id::new_id;
use crate::position::next_position;
use crate::search::sync_card_search_row;
use crate::{now_ms, Card, CardCreateDraft, CardPatch, Store};

use super::create::resolve_create_column;
use super::fields::{apply_execution_metadata, apply_workflow_fields};
use super::{load_card_by_id_tx, touch_card};

impl Store {
    pub fn create_cards_from_plan(
        &mut self,
        board_id: &str,
        drafts: &[CardCreateDraft],
        actor: &str,
    ) -> Result<Vec<Card>> {
        if drafts.is_empty() {
            return Ok(Vec::new());
        }
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        validate_plan_drafts(&tx, board_id, drafts)?;

        let (prefix, seq): (String, i64) = tx.query_row(
            "SELECT key_prefix, card_seq FROM boards WHERE id = ?1",
            params![board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        let mut alias_to_key = HashMap::new();
        let mut created = Vec::new();
        let ts = now_ms();
        for (idx, draft) in drafts.iter().enumerate() {
            let column_id = resolve_create_column(&tx, board_id, draft.column.as_deref())?;
            let key = format!("{prefix}-{}", seq + idx as i64 + 1);
            let id = new_id();
            insert_plan_card(&tx, board_id, &column_id, &id, &key, draft, ts)?;
            apply_plan_card_fields(&tx, &id, draft, ts)?;
            log_activity(&tx, &id, actor, "create", &key)?;
            sync_card_search_row(&tx, &id)?;
            alias_to_key.insert(key.clone(), key.clone());
            if let Some(alias) = draft.alias.as_deref() {
                alias_to_key.insert(alias.to_string(), key.clone());
            }
            created.push((id, key, draft.depends_on.clone()));
        }

        tx.execute(
            "UPDATE boards SET card_seq = ?1, updated_at = ?2 WHERE id = ?3",
            params![seq + drafts.len() as i64, ts, board_id],
        )?;

        apply_plan_dependencies(&tx, board_id, &created, &alias_to_key, actor)?;

        let cards = created
            .iter()
            .map(|(id, _, _)| load_card_by_id_tx(&tx, id))
            .collect::<Result<Vec<_>>>()?;
        tx.commit()?;
        Ok(cards)
    }
}

fn validate_plan_drafts(
    tx: &Transaction<'_>,
    board_id: &str,
    drafts: &[CardCreateDraft],
) -> Result<()> {
    let mut aliases = HashSet::new();
    for (idx, draft) in drafts.iter().enumerate() {
        if draft.title.trim().is_empty() {
            return Err(anyhow!("cards[{idx}].title must not be empty"));
        }
        if let Some(alias) = draft.alias.as_deref() {
            if alias.trim().is_empty() {
                return Err(anyhow!("cards[{idx}].alias must not be empty"));
            }
            if !aliases.insert(alias.to_string()) {
                return Err(anyhow!("duplicate card alias '{alias}'"));
            }
            if card_exists_tx(tx, board_id, alias)? {
                return Err(anyhow!(
                    "cards[{idx}].alias conflicts with existing card key '{alias}'"
                ));
            }
        }
    }
    Ok(())
}

fn insert_plan_card(
    tx: &Transaction<'_>,
    board_id: &str,
    column_id: &str,
    id: &str,
    key: &str,
    draft: &CardCreateDraft,
    ts: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO cards
           (id, board_id, column_id, key_text, title, body, status, priority,
            assignee, due_date, position, created_at, updated_at, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', 1, NULL, NULL, ?7, ?8, ?8, NULL)",
        params![
            id,
            board_id,
            column_id,
            key,
            draft.title.trim(),
            draft.body,
            next_position(tx, column_id)?,
            ts
        ],
    )?;
    Ok(())
}

fn apply_plan_card_fields(
    tx: &Transaction<'_>,
    card_id: &str,
    draft: &CardCreateDraft,
    ts: i64,
) -> Result<()> {
    let patch = CardPatch {
        next_action: draft.next_action.clone(),
        acceptance_criteria: draft.acceptance_criteria.clone(),
        agent_weight: draft.agent_weight,
        agent_effort: draft.agent_effort.clone(),
        suggested_model: draft.suggested_model.clone(),
        expected_tokens: draft.expected_tokens,
        human_intervention: draft.human_intervention.clone(),
        ..Default::default()
    };
    apply_workflow_fields(tx, card_id, &patch)?;
    apply_execution_metadata(tx, card_id, &patch)?;
    touch_card(tx, card_id, ts)?;
    Ok(())
}

fn apply_plan_dependencies(
    tx: &Transaction<'_>,
    board_id: &str,
    created: &[(String, String, Vec<String>)],
    alias_to_key: &HashMap<String, String>,
    actor: &str,
) -> Result<()> {
    for (_, key, dependencies) in created {
        if dependencies.is_empty() {
            continue;
        }
        let upstream = dependencies
            .iter()
            .map(|dependency| {
                if let Some(key) = alias_to_key.get(dependency) {
                    Ok(key.clone())
                } else if card_exists_tx(tx, board_id, dependency)? {
                    Ok(dependency.clone())
                } else {
                    Err(anyhow!(
                        "depends_on references unknown alias or key '{dependency}'"
                    ))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Store::set_card_dependencies_in_tx(tx, board_id, key, &upstream, actor)?;
    }
    Ok(())
}

fn card_exists_tx(tx: &Transaction<'_>, board_id: &str, key: &str) -> Result<bool> {
    let exists = tx
        .query_row(
            "SELECT 1 FROM cards WHERE board_id = ?1 AND key_text = ?2",
            params![board_id, key],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}
