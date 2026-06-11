use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use std::collections::{HashMap, HashSet};

use crate::activity::{log_activity, log_activity_payload};
use crate::id::new_id;
use crate::labels::upsert_label;
use crate::position::next_position;
use crate::rows::{
    row_to_card, CARD_SELECT_BY_ID, CARD_SELECT_BY_KEY, CARD_SELECT_PREFIX_WHERE_BOARD,
};
use crate::search::sync_card_search_row;
use crate::text::trimmed_optional;
use crate::{now_ms, parse_date, Card, CardCreateDraft, CardPatch, Store};

mod claim;

use claim::apply_claim_patch;

impl Store {
    /// All non-archived cards on a board, ordered by column then position.
    pub fn cards(&self, board_id: &str) -> Result<Vec<Card>> {
        let mut stmt = self.conn.prepare(CARD_SELECT_PREFIX_WHERE_BOARD)?;
        let rows = stmt
            .query_map(params![board_id], row_to_card)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn card_by_key(&self, board_id: &str, key: &str) -> Result<Option<Card>> {
        self.conn
            .query_row(CARD_SELECT_BY_KEY, params![board_id, key], row_to_card)
            .optional()
            .map_err(Into::into)
    }

    pub fn card_by_id(&self, card_id: &str) -> Result<Option<Card>> {
        self.conn
            .query_row(CARD_SELECT_BY_ID, params![card_id], row_to_card)
            .optional()
            .map_err(Into::into)
    }

    /// Create a card in the named column (defaults to the first column).
    pub fn create_card(
        &mut self,
        board_id: &str,
        column_name: Option<&str>,
        title: &str,
        body: &str,
        actor: &str,
    ) -> Result<Card> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let column_id: String = match column_name {
            Some(name) => tx
                .query_row(
                    "SELECT id FROM columns WHERE board_id = ?1 AND name = ?2",
                    params![board_id, name],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| anyhow!("no column named '{name}'"))?,
            None => tx.query_row(
                "SELECT id FROM columns WHERE board_id = ?1 ORDER BY sort_order LIMIT 1",
                params![board_id],
                |r| r.get(0),
            )?,
        };

        let (prefix, seq): (String, i64) = tx.query_row(
            "SELECT key_prefix, card_seq FROM boards WHERE id = ?1",
            params![board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let next_seq = seq + 1;
        let key = format!("{prefix}-{next_seq}");
        let position = next_position(&tx, &column_id)?;
        let ts = now_ms();
        let id = new_id();

        tx.execute(
            "INSERT INTO cards
               (id, board_id, column_id, key_text, title, body, status, priority,
                assignee, due_date, position, created_at, updated_at, archived_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', 1, NULL, NULL, ?7, ?8, ?8, NULL)",
            params![id, board_id, column_id, key, title, body, position, ts],
        )?;
        tx.execute(
            "UPDATE boards SET card_seq = ?1, updated_at = ?2 WHERE id = ?3",
            params![next_seq, ts, board_id],
        )?;
        log_activity(&tx, &id, actor, "create", &key)?;
        sync_card_search_row(&tx, &id)?;
        tx.commit()?;

        self.card_by_key(board_id, &key)?
            .ok_or_else(|| anyhow!("card disappeared after insert"))
    }

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
                if card_exists_tx(&tx, board_id, alias)? {
                    return Err(anyhow!(
                        "cards[{idx}].alias conflicts with existing card key '{alias}'"
                    ));
                }
            }
        }

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
                    next_position(&tx, &column_id)?,
                    ts
                ],
            )?;
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
            apply_workflow_fields(&tx, &id, &patch)?;
            apply_execution_metadata(&tx, &id, &patch)?;
            touch_card(&tx, &id, ts)?;
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

        for (_, key, dependencies) in &created {
            if dependencies.is_empty() {
                continue;
            }
            let upstream = dependencies
                .iter()
                .map(|dependency| {
                    if let Some(key) = alias_to_key.get(dependency) {
                        Ok(key.clone())
                    } else if card_exists_tx(&tx, board_id, dependency)? {
                        Ok(dependency.clone())
                    } else {
                        Err(anyhow!(
                            "depends_on references unknown alias or key '{dependency}'"
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            Store::set_card_dependencies_in_tx(&tx, board_id, key, &upstream, actor)?;
        }

        let cards = created
            .iter()
            .map(|(id, _, _)| load_card_by_id_tx(&tx, id))
            .collect::<Result<Vec<_>>>()?;
        tx.commit()?;
        Ok(cards)
    }

    /// Apply a partial update to a card identified by its key.
    pub fn update_card(
        &mut self,
        board_id: &str,
        key: &str,
        patch: &CardPatch,
        actor: &str,
    ) -> Result<Card> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let state = load_card_update_state(&tx, board_id, key)?;
        let undo = load_card_by_id_tx(&tx, &state.id)?;
        let ts = checked_update_timestamp(&state, patch, key)?;

        apply_scalar_fields(&tx, &state.id, patch, ts)?;
        let mut target_board_id = board_id.to_string();
        let board_move =
            apply_board_rehome(&tx, board_id, &mut target_board_id, &state.id, key, patch)?;
        apply_column_move(&tx, &target_board_id, &state.id, patch)?;
        apply_label_changes(&tx, &state.id, patch, ts)?;
        apply_due_date(&tx, &state.id, patch)?;
        apply_workflow_fields(&tx, &state.id, patch)?;
        apply_execution_metadata(&tx, &state.id, patch)?;
        apply_claim_patch(&tx, &state, patch, key, ts)?;
        touch_card(&tx, &state.id, ts)?;
        sync_card_search_row(&tx, &state.id)?;
        if let Some(board_move) = board_move {
            log_activity_payload(
                &tx,
                &state.id,
                actor,
                "move_board",
                board_move.into_payload(),
            )?;
        }
        log_activity_payload(
            &tx,
            &state.id,
            actor,
            "update",
            serde_json::json!({
                "detail": key,
                "undo": undo,
            }),
        )?;
        tx.commit()?;

        self.card_by_id(&state.id)?
            .ok_or_else(|| anyhow!("card disappeared after update"))
    }

    /// Undo the latest card update currently visible on a board. This is a
    /// one-step safety net for accidental archive/complete/move/edit actions;
    /// hard deletes and board/column structure changes remain intentionally
    /// irreversible.
    pub fn undo_last_card_update(&mut self, board_id: &str, actor: &str) -> Result<Option<Card>> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let last_undo_id: i64 = tx.query_row(
            "SELECT COALESCE(MAX(id), 0) FROM activity_logs WHERE action = 'undo'",
            [],
            |r| r.get(0),
        )?;
        let mut stmt = tx.prepare(
            "SELECT al.id, al.card_id, al.payload_json
               FROM activity_logs al
               JOIN cards c ON c.id = al.card_id
              WHERE c.board_id = ?1
                AND al.action = 'update'
                AND al.id > ?2
              ORDER BY al.id DESC
              LIMIT 50",
        )?;
        let candidates = stmt
            .query_map(params![board_id, last_undo_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        let Some((_, card_id, undo)) = candidates.into_iter().find_map(|(id, card_id, payload)| {
            let payload = serde_json::from_str::<serde_json::Value>(&payload).ok()?;
            let undo = serde_json::from_value::<Card>(payload.get("undo")?.clone()).ok()?;
            Some((id, card_id, undo))
        }) else {
            tx.commit()?;
            return Ok(None);
        };

        restore_card_snapshot(&tx, &undo)?;
        sync_card_search_row(&tx, &card_id)?;
        log_activity_payload(
            &tx,
            &card_id,
            actor,
            "undo",
            serde_json::json!({
                "detail": format!("restored {}", undo.key),
            }),
        )?;
        tx.commit()?;

        self.card_by_id(&card_id)?
            .map(Some)
            .ok_or_else(|| anyhow!("card disappeared after undo"))
    }

    /// Move a card to the end of an adjacent column by name. Convenience for the
    /// TUI's h/l keys; equally expressible via `update_card` with `column`.
    pub fn move_card(
        &mut self,
        board_id: &str,
        key: &str,
        column_name: &str,
        actor: &str,
    ) -> Result<Card> {
        let patch = CardPatch {
            column: Some(column_name.to_string()),
            ..Default::default()
        };
        self.update_card(board_id, key, &patch, actor)
    }

    /// Move a card up (-1) or down (+1) within its own column by swapping its
    /// fractional position with the adjacent card. No-op at the ends.
    pub fn reorder_card(&mut self, board_id: &str, key: &str, dir: i32) -> Result<()> {
        if dir == 0 {
            return Ok(());
        }
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let (card_id, column_id, position): (String, String, f64) = tx
            .query_row(
                "SELECT id, column_id, position FROM cards
                 WHERE board_id = ?1 AND key_text = ?2 AND archived_at IS NULL",
                params![board_id, key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no card '{key}'"))?;

        let neighbour: Option<(String, f64)> = if dir < 0 {
            tx.query_row(
                "SELECT id, position FROM cards
                 WHERE column_id = ?1 AND archived_at IS NULL AND position < ?2
                 ORDER BY position DESC LIMIT 1",
                params![column_id, position],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
        } else {
            tx.query_row(
                "SELECT id, position FROM cards
                 WHERE column_id = ?1 AND archived_at IS NULL AND position > ?2
                 ORDER BY position ASC LIMIT 1",
                params![column_id, position],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
        };

        if let Some((other_id, other_pos)) = neighbour {
            tx.execute(
                "UPDATE cards SET position = ?1 WHERE id = ?2",
                params![other_pos, card_id],
            )?;
            tx.execute(
                "UPDATE cards SET position = ?1 WHERE id = ?2",
                params![position, other_id],
            )?;
            tx.commit()?;
        }
        Ok(())
    }
}

pub(super) struct CardUpdateState {
    pub(super) id: String,
    updated_at: i64,
    pub(super) claimed_by: Option<String>,
    pub(super) lease_expires_at: Option<i64>,
}

fn load_card_update_state(
    tx: &Transaction<'_>,
    board_id: &str,
    key: &str,
) -> Result<CardUpdateState> {
    tx.query_row(
        "SELECT id, updated_at, claimed_by, lease_expires_at FROM cards WHERE board_id = ?1 AND key_text = ?2",
        params![board_id, key],
        |r| {
            Ok(CardUpdateState {
                id: r.get(0)?,
                updated_at: r.get(1)?,
                claimed_by: r.get(2)?,
                lease_expires_at: r.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("no card '{key}'"))
}

fn load_card_by_id_tx(tx: &Transaction<'_>, card_id: &str) -> Result<Card> {
    tx.query_row(CARD_SELECT_BY_ID, params![card_id], row_to_card)
        .optional()?
        .ok_or_else(|| anyhow!("no card id '{card_id}'"))
}

fn resolve_create_column(
    tx: &Transaction<'_>,
    board_id: &str,
    column_name: Option<&str>,
) -> Result<String> {
    match column_name {
        Some(name) => tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 AND name = ?2",
                params![board_id, name],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no column named '{name}'")),
        None => tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 ORDER BY sort_order LIMIT 1",
                params![board_id],
                |r| r.get(0),
            )
            .map_err(Into::into),
    }
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

fn restore_card_snapshot(tx: &Transaction<'_>, card: &Card) -> Result<()> {
    tx.execute(
        "UPDATE cards
            SET board_id = ?1,
                column_id = ?2,
                key_text = ?3,
                title = ?4,
                body = ?5,
                status = ?6,
                priority = ?7,
                assignee = ?8,
                due_date = ?9,
                next_action = ?10,
                blocked_reason = ?11,
                acceptance_criteria = ?12,
                handoff_note = ?13,
                last_verification = ?14,
                agent_weight = ?15,
                agent_effort = ?16,
                suggested_model = ?17,
                expected_tokens = ?18,
                human_intervention = ?19,
                claimed_by = ?20,
                claimed_at = ?21,
                lease_expires_at = ?22,
                position = ?23,
                updated_at = ?24,
                archived_at = ?25
          WHERE id = ?26",
        params![
            &card.board_id,
            &card.column_id,
            &card.key,
            &card.title,
            &card.body,
            &card.agent_state,
            card.priority,
            &card.assignee,
            card.due_date,
            &card.next_action,
            &card.blocked_reason,
            &card.acceptance_criteria,
            &card.handoff_note,
            &card.last_verification,
            card.agent_weight,
            &card.agent_effort,
            &card.suggested_model,
            card.expected_tokens,
            &card.human_intervention,
            &card.claimed_by,
            card.claimed_at,
            card.lease_expires_at,
            card.position,
            now_ms().max(card.updated_at + 1),
            card.archived_at,
            &card.id,
        ],
    )?;
    Ok(())
}

fn checked_update_timestamp(state: &CardUpdateState, patch: &CardPatch, key: &str) -> Result<i64> {
    if let Some(expected) = patch.expected_updated_at {
        if expected != state.updated_at {
            return Err(anyhow!(
                "stale update for '{key}': expected updated_at={expected}, actual={}",
                state.updated_at
            ));
        }
    }
    let mut ts = now_ms();
    if ts <= state.updated_at {
        ts = state.updated_at + 1;
    }
    Ok(ts)
}

fn apply_scalar_fields(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
    ts: i64,
) -> Result<()> {
    if let Some(title) = &patch.title {
        tx.execute(
            "UPDATE cards SET title = ?1 WHERE id = ?2",
            params![title, card_id],
        )?;
    }
    if let Some(body) = &patch.body {
        tx.execute(
            "UPDATE cards SET body = ?1 WHERE id = ?2",
            params![body, card_id],
        )?;
    }
    if let Some(agent_state) = &patch.agent_state {
        tx.execute(
            "UPDATE cards SET status = ?1 WHERE id = ?2",
            params![agent_state, card_id],
        )?;
    }
    if let Some(priority) = patch.priority {
        tx.execute(
            "UPDATE cards SET priority = ?1 WHERE id = ?2",
            params![priority, card_id],
        )?;
    }
    if patch.assignee.is_some() {
        tx.execute(
            "UPDATE cards SET assignee = ?1 WHERE id = ?2",
            params![patch.assignee, card_id],
        )?;
    }
    if let Some(archived) = patch.archived {
        let val = if archived { Some(ts) } else { None };
        tx.execute(
            "UPDATE cards SET archived_at = ?1 WHERE id = ?2",
            params![val, card_id],
        )?;
    }
    Ok(())
}

fn apply_column_move(
    tx: &Transaction<'_>,
    board_id: &str,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(column_name) = &patch.column {
        let column_id: String = tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 AND name = ?2",
                params![board_id, column_name],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no column named '{column_name}'"))?;
        let position = next_position(tx, &column_id)?;
        tx.execute(
            "UPDATE cards SET column_id = ?1, position = ?2 WHERE id = ?3",
            params![column_id, position, card_id],
        )?;
    }
    Ok(())
}

fn apply_board_rehome(
    tx: &Transaction<'_>,
    source_board_id: &str,
    target_board_id: &mut String,
    card_id: &str,
    old_key: &str,
    patch: &CardPatch,
) -> Result<Option<BoardMoveActivity>> {
    let Some(dest_board_id) = &patch.move_to_board else {
        return Ok(None);
    };
    if dest_board_id == source_board_id {
        return Ok(None);
    }

    let (source_name, source_slug): (String, String) = tx
        .query_row(
            "SELECT name, slug FROM boards WHERE id = ?1",
            params![source_board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no board '{source_board_id}'"))?;

    let (dest_name, dest_slug, prefix, seq): (String, String, String, i64) = tx
        .query_row(
            "SELECT name, slug, key_prefix, card_seq FROM boards WHERE id = ?1",
            params![dest_board_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no board '{dest_board_id}'"))?;

    let new_key = format!("{}-{}", prefix, seq + 1);
    tx.execute(
        "UPDATE boards SET card_seq = ?1 WHERE id = ?2",
        params![seq + 1, dest_board_id],
    )?;

    let dest_col: String = tx
        .query_row(
            "SELECT id FROM columns WHERE board_id = ?1 ORDER BY sort_order LIMIT 1",
            params![dest_board_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("destination board has no columns"))?;

    tx.execute(
        "UPDATE cards SET board_id = ?1, column_id = ?2, position = ?3, key_text = ?4 WHERE id = ?5",
        params![
            dest_board_id,
            dest_col,
            next_position(tx, &dest_col)?,
            new_key,
            card_id
        ],
    )?;
    *target_board_id = dest_board_id.to_string();
    Ok(Some(BoardMoveActivity {
        old_key: old_key.to_string(),
        new_key,
        source_board_id: source_board_id.to_string(),
        source_board_name: source_name,
        source_board_slug: source_slug,
        destination_board_id: dest_board_id.to_string(),
        destination_board_name: dest_name,
        destination_board_slug: dest_slug,
    }))
}

#[derive(Debug)]
struct BoardMoveActivity {
    old_key: String,
    new_key: String,
    source_board_id: String,
    source_board_name: String,
    source_board_slug: String,
    destination_board_id: String,
    destination_board_name: String,
    destination_board_slug: String,
}

impl BoardMoveActivity {
    fn into_payload(self) -> serde_json::Value {
        serde_json::json!({
            "old_key": self.old_key,
            "new_key": self.new_key,
            "source_board": {
                "id": self.source_board_id,
                "name": self.source_board_name,
                "slug": self.source_board_slug,
            },
            "destination_board": {
                "id": self.destination_board_id,
                "name": self.destination_board_name,
                "slug": self.destination_board_slug,
            },
        })
    }
}

fn apply_label_changes(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
    ts: i64,
) -> Result<()> {
    if let Some(names) = &patch.add_labels {
        for name in names {
            let label_id = upsert_label(tx, name)?;
            tx.execute(
                "INSERT OR IGNORE INTO card_labels (card_id, label_id) VALUES (?1, ?2)",
                params![card_id, label_id],
            )?;
            tx.execute(
                "UPDATE labels SET last_used_at = ?1 WHERE id = ?2",
                params![ts, label_id],
            )?;
        }
    }
    if let Some(names) = &patch.remove_labels {
        for name in names {
            tx.execute(
                "DELETE FROM card_labels WHERE card_id = ?1 AND label_id =
                   (SELECT id FROM labels WHERE name = ?2)",
                params![card_id, name],
            )?;
        }
    }
    Ok(())
}

fn apply_due_date(tx: &Transaction<'_>, card_id: &str, patch: &CardPatch) -> Result<()> {
    if let Some(due) = &patch.due {
        let val: Option<i64> = if due.trim().is_empty() {
            None
        } else {
            Some(parse_date(due)?)
        };
        tx.execute(
            "UPDATE cards SET due_date = ?1 WHERE id = ?2",
            params![val, card_id],
        )?;
    }
    Ok(())
}

fn apply_workflow_fields(tx: &Transaction<'_>, card_id: &str, patch: &CardPatch) -> Result<()> {
    if let Some(next_action) = &patch.next_action {
        tx.execute(
            "UPDATE cards SET next_action = ?1 WHERE id = ?2",
            params![trimmed_optional(next_action), card_id],
        )?;
    }
    if let Some(blocked_reason) = &patch.blocked_reason {
        tx.execute(
            "UPDATE cards SET blocked_reason = ?1 WHERE id = ?2",
            params![trimmed_optional(blocked_reason), card_id],
        )?;
    }
    if let Some(acceptance_criteria) = &patch.acceptance_criteria {
        tx.execute(
            "UPDATE cards SET acceptance_criteria = ?1 WHERE id = ?2",
            params![trimmed_optional(acceptance_criteria), card_id],
        )?;
    }
    if let Some(handoff_note) = &patch.handoff_note {
        tx.execute(
            "UPDATE cards SET handoff_note = ?1 WHERE id = ?2",
            params![trimmed_optional(handoff_note), card_id],
        )?;
    }
    if let Some(last_verification) = &patch.last_verification {
        tx.execute(
            "UPDATE cards SET last_verification = ?1 WHERE id = ?2",
            params![trimmed_optional(last_verification), card_id],
        )?;
    }
    Ok(())
}

fn apply_execution_metadata(tx: &Transaction<'_>, card_id: &str, patch: &CardPatch) -> Result<()> {
    if let Some(agent_weight) = patch.agent_weight {
        if let Some(weight) = agent_weight {
            if !(1..=5).contains(&weight) {
                return Err(anyhow!("agent_weight must be between 1 and 5"));
            }
        }
        tx.execute(
            "UPDATE cards SET agent_weight = ?1 WHERE id = ?2",
            params![agent_weight, card_id],
        )?;
    }
    if let Some(agent_effort) = &patch.agent_effort {
        tx.execute(
            "UPDATE cards SET agent_effort = ?1 WHERE id = ?2",
            params![trimmed_optional(agent_effort), card_id],
        )?;
    }
    if let Some(suggested_model) = &patch.suggested_model {
        tx.execute(
            "UPDATE cards SET suggested_model = ?1 WHERE id = ?2",
            params![trimmed_optional(suggested_model), card_id],
        )?;
    }
    if let Some(expected_tokens) = patch.expected_tokens {
        if let Some(tokens) = expected_tokens {
            if tokens <= 0 {
                return Err(anyhow!("expected_tokens must be positive"));
            }
        }
        tx.execute(
            "UPDATE cards SET expected_tokens = ?1 WHERE id = ?2",
            params![expected_tokens, card_id],
        )?;
    }
    if let Some(human_intervention) = &patch.human_intervention {
        let value = trimmed_optional(human_intervention);
        if let Some(value) = value {
            match value {
                "none" | "review" | "decision" | "execution" => {}
                _ => {
                    return Err(anyhow!(
                        "human_intervention must be none, review, decision, or execution"
                    ))
                }
            }
        }
        tx.execute(
            "UPDATE cards SET human_intervention = ?1 WHERE id = ?2",
            params![value, card_id],
        )?;
    }
    Ok(())
}

fn touch_card(tx: &Transaction<'_>, card_id: &str, ts: i64) -> Result<()> {
    tx.execute(
        "UPDATE cards SET updated_at = ?1 WHERE id = ?2",
        params![ts, card_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_claim_releases_existing_claim() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "claim me", "", "test")
            .unwrap();
        let agent = store.register_agent("agent-a", None, None, None).unwrap();

        store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    claim: Some(agent.registration.assigned_identity.clone()),
                    claim_token: Some(agent.claim_token.clone()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();
        let released = store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    claim: Some(" ".to_string()),
                    claim_token: Some(agent.claim_token),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        assert_eq!(released.claimed_by, None);
        assert_eq!(released.claimed_at, None);
        assert_eq!(released.lease_expires_at, None);
    }

    #[test]
    fn blank_workflow_fields_clear_optional_values() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "workflow", "", "test")
            .unwrap();

        store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    next_action: Some("Run tests".to_string()),
                    blocked_reason: Some("Waiting".to_string()),
                    acceptance_criteria: Some("All green".to_string()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();
        let cleared = store
            .update_card(
                &board.id,
                &card.key,
                &CardPatch {
                    next_action: Some(" ".to_string()),
                    blocked_reason: Some("".to_string()),
                    acceptance_criteria: Some(" ".to_string()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        assert_eq!(cleared.next_action, None);
        assert_eq!(cleared.blocked_reason, None);
        assert_eq!(cleared.acceptance_criteria, None);
    }
}
