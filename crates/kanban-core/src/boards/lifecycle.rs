use anyhow::{anyhow, Result};
use rusqlite::params;

use crate::{now_ms, Board, Store, PROTECTED_BOARD_SLUG};

use super::read;

pub(super) fn archive_board(store: &mut Store, board_id: &str) -> Result<()> {
    store.assert_writable()?;
    let board = read::board_by_id(&store.conn, board_id)?;
    if board.slug == PROTECTED_BOARD_SLUG {
        return Err(anyhow!("the Backlog board cannot be archived"));
    }
    store.conn.execute(
        "UPDATE boards SET archived_at = ?1 WHERE id = ?2 AND archived_at IS NULL",
        params![now_ms(), board_id],
    )?;
    Ok(())
}

pub(super) fn unarchive_board(store: &mut Store, board_id: &str) -> Result<()> {
    store.assert_writable()?;
    store.conn.execute(
        "UPDATE boards SET archived_at = NULL WHERE id = ?1",
        params![board_id],
    )?;
    Ok(())
}

pub(super) fn delete_board(store: &mut Store, board_id: &str) -> Result<()> {
    store.assert_writable()?;
    let board = read::board_by_id(&store.conn, board_id)?;
    if board.slug == PROTECTED_BOARD_SLUG {
        return Err(anyhow!("the Backlog board cannot be deleted"));
    }
    if board.archived_at.is_none() {
        return Err(anyhow!(
            "board '{}' is not archived; archive it first, then delete",
            board.slug
        ));
    }
    store
        .conn
        .execute("DELETE FROM boards WHERE id = ?1", params![board_id])?;
    Ok(())
}

pub(super) fn update_board_agent_context(
    store: &mut Store,
    board_id: &str,
    agent_context: Option<&str>,
) -> Result<Board> {
    store.assert_writable()?;
    let normalized = agent_context.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    store.conn.execute(
        "UPDATE boards SET agent_context = ?1, updated_at = ?2 WHERE id = ?3",
        params![normalized, now_ms(), board_id],
    )?;
    read::board_by_id(&store.conn, board_id)
}
