use anyhow::{anyhow, Result};
use rusqlite::{params, TransactionBehavior};

use crate::id::new_id;
use crate::{now_ms, Memory, Store};

use super::read;

pub(super) fn record_memory(
    store: &mut Store,
    title: &str,
    body: &str,
    kind: Option<&str>,
    card_key: Option<&str>,
) -> Result<Memory> {
    let title = title.trim();
    if title.is_empty() {
        return Err(anyhow!("memory title must not be empty"));
    }
    let kind = kind
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .unwrap_or("note");
    store.assert_writable()?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let seq: i64 = tx.query_row(
        "SELECT value FROM counters WHERE name = 'memory_seq'",
        [],
        |r| r.get(0),
    )?;
    let next_seq = seq + 1;
    let key = format!("M-{next_seq}");
    let ts = now_ms();
    let id = new_id();
    tx.execute(
        "INSERT INTO memories
           (id, key_text, title, body, kind, card_key, created_at, updated_at, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)",
        params![id, key, title, body, kind, card_key, ts],
    )?;
    tx.execute(
        "UPDATE counters SET value = ?1 WHERE name = 'memory_seq'",
        params![next_seq],
    )?;
    tx.commit()?;
    read::memory_by_key(&store.conn, &key)?
        .ok_or_else(|| anyhow!("memory disappeared after insert"))
}
