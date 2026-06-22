use anyhow::Result;
use rusqlite::{params, OptionalExtension, Transaction};

use crate::rows::{row_to_card, CARD_SELECT_BY_KEY};
use crate::{Card, CardDependency};

pub(super) fn load_card_by_key_tx(
    tx: &Transaction<'_>,
    board_id: &str,
    key: &str,
) -> Result<Option<Card>> {
    tx.query_row(CARD_SELECT_BY_KEY, params![board_id, key], row_to_card)
        .optional()
        .map_err(Into::into)
}

pub(super) fn load_dependencies(
    conn: &rusqlite::Connection,
    board_id: &str,
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
          WHERE d.board_id = ?1
          ORDER BY down.key_text, up.key_text",
    )?;
    collect_dependencies(&mut stmt, params![board_id])
}

pub(super) fn load_dependencies_for_downstream(
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

pub(super) fn load_upstream_cards_for_card(
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
