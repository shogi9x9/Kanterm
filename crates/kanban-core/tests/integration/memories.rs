use crate::common::{temp_db, TempDb};
use kanban_core::Store;
use rusqlite::Connection;

#[test]
fn memories_record_recall_and_persist() {
    let db = TempDb(temp_db("memories"));
    {
        let mut s = Store::open(&db.0).unwrap();
        let m = s
            .record_memory(
                "use rusqlite 0.37",
                "0.38 needs a newer rustc; pinned in Cargo.toml",
                Some("decision"),
                Some("KB-3"),
            )
            .unwrap();
        assert_eq!(m.key, "M-1");
        assert_eq!(m.kind, "decision");
        let m2 = s
            .record_memory("popup over status bar", "", None, None)
            .unwrap();
        assert_eq!(m2.key, "M-2");
        assert_eq!(m2.kind, "note");
    }
    // Reopen: memories survive, keys keep counting.
    {
        let mut s = Store::open(&db.0).unwrap();
        assert_eq!(s.record_memory("third", "", None, None).unwrap().key, "M-3");

        // Substring recall over title/body, case-insensitive; newest first.
        let hits = s
            .recall_memories(Some("rustc"), None, None, 10, false)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, "M-1");
        // Filters: by card key and by kind.
        assert_eq!(
            s.recall_memories(None, Some("KB-3"), None, 10, false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            s.recall_memories(None, None, Some("decision"), 10, false)
                .unwrap()
                .len(),
            1
        );
        let all = s.recall_memories(None, None, None, 10, false).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].key, "M-3", "newest first");

        // LIKE wildcards in queries match literally, not as wildcards.
        assert_eq!(
            s.recall_memories(Some("100%"), None, None, 10, false)
                .unwrap()
                .len(),
            0
        );
    }
}

#[test]
fn memories_update_and_archive() {
    let db = TempDb(temp_db("mem-update"));
    let mut s = Store::open(&db.0).unwrap();
    s.record_memory("draft", "old body", None, None).unwrap();

    let patched = s
        .update_memory(
            "M-1",
            &kanban_core::MemoryPatch {
                title: Some("final".into()),
                body: Some("new body".into()),
                kind: Some("decision".into()),
                card_key: Some("KB-9".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(patched.title, "final");
    assert_eq!(patched.body, "new body");
    assert_eq!(patched.card_key.as_deref(), Some("KB-9"));

    // Clearing the card link with "".
    let cleared = s
        .update_memory(
            "M-1",
            &kanban_core::MemoryPatch {
                card_key: Some("".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(cleared.card_key.is_none());

    // Archive hides from recall unless include_archived.
    s.update_memory(
        "M-1",
        &kanban_core::MemoryPatch {
            archived: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s
        .recall_memories(None, None, None, 10, false)
        .unwrap()
        .is_empty());
    assert_eq!(
        s.recall_memories(None, None, None, 10, true).unwrap().len(),
        1
    );

    // Unknown key errors cleanly.
    assert!(s.update_memory("M-999", &Default::default()).is_err());
}

#[test]
fn memories_track_recall_and_purge_unrecalled() {
    let db = TempDb(temp_db("mem-retention"));
    let mut s = Store::open(&db.0).unwrap();
    s.record_memory("keep", "returned to an agent", None, None)
        .unwrap();
    s.record_memory("drop", "never recalled", None, None)
        .unwrap();

    s.mark_memories_recalled(["M-1"].iter().copied()).unwrap();
    let kept = s.memory_by_key("M-1").unwrap().unwrap();
    assert_eq!(kept.recall_count, 1);
    assert!(kept.last_recalled_at.is_some());

    let deleted = s.purge_unrecalled_memories_older_than(i64::MAX).unwrap();
    assert_eq!(deleted, 1);
    assert!(s.memory_by_key("M-1").unwrap().is_some());
    assert!(s.memory_by_key("M-2").unwrap().is_none());
}

#[test]
fn memory_gc_runs_on_open_and_keeps_recalled_memories() {
    let db = TempDb(temp_db("mem-gc-open"));
    {
        let mut s = Store::open(&db.0).unwrap();
        s.record_memory("keep", "was recalled", None, None).unwrap();
        s.record_memory("drop", "never recalled", None, None)
            .unwrap();
        s.mark_memories_recalled(["M-1"].iter().copied()).unwrap();
    }
    {
        let conn = Connection::open(&db.0).unwrap();
        conn.execute("UPDATE memories SET created_at = 1, updated_at = 1", [])
            .unwrap();
        conn.execute(
            "UPDATE counters SET value = 0 WHERE name = 'memory_gc_last_run'",
            [],
        )
        .unwrap();
    }
    {
        let s = Store::open(&db.0).unwrap();
        assert!(s.memory_by_key("M-1").unwrap().is_some());
        assert!(s.memory_by_key("M-2").unwrap().is_none());
    }
}
