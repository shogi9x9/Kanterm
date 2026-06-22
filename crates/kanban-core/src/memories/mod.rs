use anyhow::Result;

use crate::{Memory, MemoryPatch, Store};

mod gc;
mod read;
mod recall;
mod record;
mod update;

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
        record::record_memory(self, title, body, kind, card_key)
    }

    pub fn memory_by_key(&self, key: &str) -> Result<Option<Memory>> {
        read::memory_by_key(&self.conn, key)
    }

    /// Mark memories as referenced by an agent recall. This is deliberately not
    /// called by read-only TUI browsing; opening the browser should not keep every
    /// memory alive forever.
    pub fn mark_memories_recalled<'a, I>(&mut self, keys: I) -> Result<()>
    where
        I: IntoIterator<Item = &'a str>,
    {
        recall::mark_memories_recalled(self, keys)
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
        recall::recall_memories(&self.conn, query, card_key, kind, limit, include_archived)
    }

    pub fn update_memory(&mut self, key: &str, patch: &MemoryPatch) -> Result<Memory> {
        update::update_memory(self, key, patch)
    }

    /// Delete active or archived memories that have never been recalled and are
    /// older than the retention window. Returns the number of deleted rows.
    pub fn purge_unrecalled_memories_older_than(&mut self, cutoff_ms: i64) -> Result<usize> {
        gc::purge_unrecalled_memories_older_than(self, cutoff_ms)
    }

    /// Opportunistic monthly GC: every opener checks a cheap counter and only one
    /// process that wins BEGIN IMMEDIATE performs the retention pass.
    pub fn run_due_memory_gc(&mut self) -> Result<usize> {
        gc::run_due_memory_gc(self)
    }
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
