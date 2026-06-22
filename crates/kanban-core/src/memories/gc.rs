use anyhow::Result;
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{now_ms, Store, MS_PER_DAY};

const MEMORY_RETENTION_MS: i64 = 30 * MS_PER_DAY;
const MEMORY_GC_INTERVAL_MS: i64 = 30 * MS_PER_DAY;
const PURGE_UNRECALLED_SQL: &str = "DELETE FROM memories
      WHERE recall_count = 0
        AND last_recalled_at IS NULL
        AND created_at < ?1";

pub(super) fn purge_unrecalled_memories_older_than(
    store: &mut Store,
    cutoff_ms: i64,
) -> Result<usize> {
    store.assert_writable()?;
    purge_unrecalled_on_connection(&store.conn, cutoff_ms)
}

pub(super) fn run_due_memory_gc(store: &mut Store) -> Result<usize> {
    store.assert_writable()?;
    let now = now_ms();
    let last_run: i64 = store.conn.query_row(
        "SELECT value FROM counters WHERE name = 'memory_gc_last_run'",
        [],
        |r| r.get(0),
    )?;
    if !memory_gc_due(last_run, now) {
        return Ok(0);
    }
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let last_run: i64 = tx.query_row(
        "SELECT value FROM counters WHERE name = 'memory_gc_last_run'",
        [],
        |r| r.get(0),
    )?;
    if !memory_gc_due(last_run, now) {
        tx.commit()?;
        return Ok(0);
    }
    let cutoff = now.saturating_sub(MEMORY_RETENTION_MS);
    let deleted = purge_unrecalled_in_tx(&tx, cutoff)?;
    update_memory_gc_last_run(&tx, now)?;
    tx.commit()?;
    Ok(deleted)
}

fn memory_gc_due(last_run: i64, now: i64) -> bool {
    now.saturating_sub(last_run) >= MEMORY_GC_INTERVAL_MS
}

fn purge_unrecalled_on_connection(conn: &rusqlite::Connection, cutoff_ms: i64) -> Result<usize> {
    Ok(conn.execute(PURGE_UNRECALLED_SQL, params![cutoff_ms])?)
}

fn purge_unrecalled_in_tx(tx: &Transaction<'_>, cutoff_ms: i64) -> Result<usize> {
    Ok(tx.execute(PURGE_UNRECALLED_SQL, params![cutoff_ms])?)
}

fn update_memory_gc_last_run(tx: &Transaction<'_>, now: i64) -> Result<()> {
    tx.execute(
        "UPDATE counters SET value = ?1 WHERE name = 'memory_gc_last_run'",
        params![now],
    )?;
    Ok(())
}
