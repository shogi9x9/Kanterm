use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::{now_ms, Memory, MemoryPatch, Store};

use super::read;

pub(super) fn update_memory(store: &mut Store, key: &str, patch: &MemoryPatch) -> Result<Memory> {
    store.assert_writable()?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (id, current_updated_at) = load_memory_update_target(&tx, key)?;
    let ts = next_memory_update_ts(current_updated_at);
    apply_memory_patch(&tx, &id, patch, ts)?;
    touch_memory(&tx, &id, ts)?;
    tx.commit()?;
    read::memory_by_key(&store.conn, key)?.ok_or_else(|| anyhow!("memory disappeared after update"))
}

fn load_memory_update_target(tx: &Transaction<'_>, key: &str) -> Result<(String, i64)> {
    tx.query_row(
        "SELECT id, updated_at FROM memories WHERE key_text = ?1",
        params![key],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| anyhow!("no memory '{key}'"))
}

fn next_memory_update_ts(current_updated_at: i64) -> i64 {
    let mut ts = now_ms();
    if ts <= current_updated_at {
        ts = current_updated_at + 1;
    }
    ts
}

fn apply_memory_patch(tx: &Transaction<'_>, id: &str, patch: &MemoryPatch, ts: i64) -> Result<()> {
    if let Some(title) = &patch.title {
        let title = title.trim();
        if title.is_empty() {
            return Err(anyhow!("memory title must not be empty"));
        }
        tx.execute(
            "UPDATE memories SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
    }
    if let Some(body) = &patch.body {
        tx.execute(
            "UPDATE memories SET body = ?1 WHERE id = ?2",
            params![body, id],
        )?;
    }
    if let Some(kind) = &patch.kind {
        tx.execute(
            "UPDATE memories SET kind = ?1 WHERE id = ?2",
            params![kind.trim(), id],
        )?;
    }
    if let Some(ck) = &patch.card_key {
        let val = if ck.trim().is_empty() {
            None
        } else {
            Some(ck.trim())
        };
        tx.execute(
            "UPDATE memories SET card_key = ?1 WHERE id = ?2",
            params![val, id],
        )?;
    }
    if let Some(archived) = patch.archived {
        let val = if archived { Some(ts) } else { None };
        tx.execute(
            "UPDATE memories SET archived_at = ?1 WHERE id = ?2",
            params![val, id],
        )?;
    }
    Ok(())
}

fn touch_memory(tx: &Transaction<'_>, id: &str, ts: i64) -> Result<()> {
    tx.execute(
        "UPDATE memories SET updated_at = ?1 WHERE id = ?2",
        params![ts, id],
    )?;
    Ok(())
}
