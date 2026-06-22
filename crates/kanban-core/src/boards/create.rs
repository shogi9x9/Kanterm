use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::id::new_id;
use crate::naming::{derive_prefix, derive_slug};
use crate::{now_ms, Board, BoardColumnTemplate, Store, PROTECTED_BOARD_SLUG};

use super::read;

/// Insert a board and its default columns. Returns the board id.
pub(crate) fn insert_board(
    tx: &rusqlite::Transaction<'_>,
    name: &str,
    slug: &str,
    prefix: &str,
    columns: &[&str],
) -> Result<String> {
    let board_id = new_id();
    let ts = now_ms();
    let sort_order = tx.query_row(
        "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM boards",
        [],
        |r| r.get::<_, i64>(0),
    )?;
    tx.execute(
        "INSERT INTO boards (id, name, slug, key_prefix, card_seq, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)",
        params![board_id, name, slug, prefix, sort_order, ts],
    )?;
    for (order, col) in columns.iter().enumerate() {
        tx.execute(
            "INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            params![new_id(), board_id, col, order as i64, ts],
        )?;
    }
    Ok(board_id)
}

pub(super) fn ensure_system_board(
    store: &mut Store,
    name: &str,
    slug: &str,
    prefix: &str,
    columns: &[&str],
) -> Result<Board> {
    if let Some(b) = read::board_by_slug(&store.conn, slug)? {
        return Ok(b);
    }
    store.assert_writable()?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    // Another process may have created the board between our check and here;
    // on the unique-constraint loss, roll back and re-read its board.
    if insert_board(&tx, name, slug, prefix, columns).is_ok() {
        tx.commit()?;
    } else {
        drop(tx);
    }
    read::board_by_slug(&store.conn, slug)?.ok_or_else(|| anyhow!("board creation failed"))
}

pub(super) fn create_board(
    store: &mut Store,
    name: &str,
    template: BoardColumnTemplate,
) -> Result<Board> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("board name must not be empty"));
    }
    store.assert_writable()?;
    let prefix = derive_prefix(name);
    let base = derive_slug(name);
    if base == PROTECTED_BOARD_SLUG {
        return Err(anyhow!("Backlog is the reserved default board"));
    }
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut slug = base.clone();
    let mut n = 2;
    while tx
        .query_row(
            "SELECT 1 FROM boards WHERE slug = ?1",
            params![slug],
            |_| Ok(()),
        )
        .optional()?
        .is_some()
    {
        slug = format!("{base}-{n}");
        n += 1;
    }
    insert_board(&tx, name, &slug, &prefix, template.columns())?;
    tx.commit()?;
    read::board_by_slug(&store.conn, &slug)?.ok_or_else(|| anyhow!("board creation failed"))
}
