use anyhow::{anyhow, Result};
use rusqlite::types::ToSql;
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::id::new_id;
use crate::rows::{row_to_memory, MEMORY_COLUMNS};
use crate::text::like_escape;
use crate::{now_ms, Memory, MemoryPatch, Store, MS_PER_DAY};

const MEMORY_RETENTION_MS: i64 = 30 * MS_PER_DAY;
const MEMORY_GC_INTERVAL_MS: i64 = 30 * MS_PER_DAY;
const PURGE_UNRECALLED_SQL: &str = "DELETE FROM memories
      WHERE recall_count = 0
        AND last_recalled_at IS NULL
        AND created_at < ?1";

impl Store {
    /// Record a memory (decision/learning/context note). Allocates the next
    /// `M-N` key from the shared counter under BEGIN IMMEDIATE, same scheme as
    /// card keys.
    pub fn record_memory(
        &mut self,
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
        self.assert_writable()?;
        let tx = self
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
        self.memory_by_key(&key)?
            .ok_or_else(|| anyhow!("memory disappeared after insert"))
    }

    pub fn memory_by_key(&self, key: &str) -> Result<Option<Memory>> {
        self.conn
            .query_row(
                &format!("SELECT {MEMORY_COLUMNS} FROM memories WHERE key_text = ?1"),
                params![key],
                row_to_memory,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Mark memories as referenced by an agent recall. This is deliberately not
    /// called by read-only TUI browsing; opening the browser should not keep every
    /// memory alive forever.
    pub fn mark_memories_recalled<'a, I>(&mut self, keys: I) -> Result<()>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let keys: Vec<&str> = keys.into_iter().filter(|k| !k.trim().is_empty()).collect();
        if keys.is_empty() {
            return Ok(());
        }
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let ts = now_ms();
        for key in keys {
            tx.execute(
                "UPDATE memories
                    SET last_recalled_at = ?1,
                        recall_count = recall_count + 1
                  WHERE key_text = ?2",
                params![ts, key],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Search memories, newest first. `query` is a case-insensitive substring
    /// match over title, body and card_key; `card_key` / `kind` filter exactly.
    /// Archived memories are excluded unless `include_archived`.
    pub fn recall_memories(
        &self,
        query: Option<&str>,
        card_key: Option<&str>,
        kind: Option<&str>,
        limit: usize,
        include_archived: bool,
    ) -> Result<Vec<Memory>> {
        let (sql, args) = build_recall_query(query, card_key, kind, limit, include_archived);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(args.iter().map(|a| a.as_ref())),
                row_to_memory,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn update_memory(&mut self, key: &str, patch: &MemoryPatch) -> Result<Memory> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (id, current_updated_at) = load_memory_update_target(&tx, key)?;
        let ts = next_memory_update_ts(current_updated_at);
        apply_memory_patch(&tx, &id, patch, ts)?;
        touch_memory(&tx, &id, ts)?;
        tx.commit()?;
        self.memory_by_key(key)?
            .ok_or_else(|| anyhow!("memory disappeared after update"))
    }

    /// Delete active or archived memories that have never been recalled and are
    /// older than the retention window. Returns the number of deleted rows.
    pub fn purge_unrecalled_memories_older_than(&mut self, cutoff_ms: i64) -> Result<usize> {
        self.assert_writable()?;
        purge_unrecalled_on_connection(&self.conn, cutoff_ms)
    }

    /// Opportunistic monthly GC: every opener checks a cheap counter and only one
    /// process that wins BEGIN IMMEDIATE performs the retention pass.
    pub fn run_due_memory_gc(&mut self) -> Result<usize> {
        self.assert_writable()?;
        let now = now_ms();
        let last_run: i64 = self.conn.query_row(
            "SELECT value FROM counters WHERE name = 'memory_gc_last_run'",
            [],
            |r| r.get(0),
        )?;
        if !memory_gc_due(last_run, now) {
            return Ok(0);
        }
        let tx = self
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
}

fn build_recall_query(
    query: Option<&str>,
    card_key: Option<&str>,
    kind: Option<&str>,
    limit: usize,
    include_archived: bool,
) -> (String, Vec<Box<dyn ToSql>>) {
    let mut sql = format!(
        "SELECT {MEMORY_COLUMNS} FROM memories WHERE 1=1{}",
        if include_archived {
            ""
        } else {
            " AND archived_at IS NULL"
        }
    );
    let mut args: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        sql.push_str(&format!(
            " AND (title LIKE ?{n} ESCAPE '\\' OR body LIKE ?{n} ESCAPE '\\' \
               OR IFNULL(card_key,'') LIKE ?{n} ESCAPE '\\')",
            n = args.len() + 1
        ));
        args.push(Box::new(format!("%{}%", like_escape(q))));
    }
    if let Some(ck) = card_key.map(str::trim).filter(|c| !c.is_empty()) {
        sql.push_str(&format!(" AND card_key = ?{}", args.len() + 1));
        args.push(Box::new(ck.to_string()));
    }
    if let Some(k) = kind.map(str::trim).filter(|k| !k.is_empty()) {
        sql.push_str(&format!(" AND kind = ?{}", args.len() + 1));
        args.push(Box::new(k.to_string()));
    }
    sql.push_str(&format!(
        " ORDER BY updated_at DESC LIMIT ?{}",
        args.len() + 1
    ));
    args.push(Box::new(limit as i64));
    (sql, args)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_memories_can_include_archived() {
        let mut store = Store::open_in_memory().unwrap();
        let memory = store
            .record_memory("Archived note", "body", Some("note"), None)
            .unwrap();
        store
            .update_memory(
                &memory.key,
                &MemoryPatch {
                    archived: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(store
            .recall_memories(Some("Archived"), None, None, 10, false)
            .unwrap()
            .is_empty());
        assert_eq!(
            store
                .recall_memories(Some("Archived"), None, None, 10, true)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn recall_memories_filters_by_query_kind_and_card_key() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .record_memory(
                "Cargo mutants",
                "mutation testing",
                Some("decision"),
                Some("KB-1"),
            )
            .unwrap();
        store
            .record_memory("Release notes", "packaging", Some("note"), Some("KB-2"))
            .unwrap();

        let hits = store
            .recall_memories(Some("mutants"), Some("KB-1"), Some("decision"), 10, false)
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Cargo mutants");
    }
}
