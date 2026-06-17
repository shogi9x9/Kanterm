//! Integration tests against the public `kanban-core` API, including the
//! concurrency story (WAL + BEGIN IMMEDIATE + busy_timeout) that the TUI and the
//! MCP server rely on when they write to the same database at once.

mod common;

use common::{temp_db, TempDb};
use kanban_core::{
    card_is_stale, now_ms, BoardColumnTemplate, CardCreateDraft, CardPatch, Store,
    BACKLOG_BOARD_COLUMNS, STALE_CARD_MS,
};
use rusqlite::Connection;
use std::collections::HashSet;

fn rollback_cards_to_schema_15_and_set_user_version(path: &std::path::Path, version: i64) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;
         DROP TABLE IF EXISTS boards_rollback;
         CREATE TABLE boards_rollback (
             id          TEXT PRIMARY KEY,
             name        TEXT NOT NULL,
             slug        TEXT NOT NULL UNIQUE,
             key_prefix  TEXT NOT NULL DEFAULT 'KB',
             card_seq    INTEGER NOT NULL DEFAULT 0,
             created_at  INTEGER NOT NULL,
             updated_at  INTEGER NOT NULL,
             archived_at INTEGER,
             sort_order  INTEGER NOT NULL DEFAULT 0
         );
         INSERT INTO boards_rollback
             (id, name, slug, key_prefix, card_seq, created_at, updated_at, archived_at, sort_order)
         SELECT id, name, slug, key_prefix, card_seq, created_at, updated_at, archived_at, sort_order
           FROM boards;
         DROP TABLE boards;
         ALTER TABLE boards_rollback RENAME TO boards;
         CREATE INDEX idx_boards_sort_order ON boards(sort_order);
         DROP TABLE IF EXISTS card_dependencies;
         DROP TABLE IF EXISTS cards_rollback;
         CREATE TABLE cards_rollback (
             id          TEXT PRIMARY KEY,
             board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
             column_id   TEXT NOT NULL REFERENCES columns(id) ON DELETE CASCADE,
             key_text    TEXT NOT NULL,
             title       TEXT NOT NULL,
             body        TEXT NOT NULL DEFAULT '',
             status      TEXT NOT NULL DEFAULT 'open',
             priority    INTEGER NOT NULL DEFAULT 1,
             assignee    TEXT,
             due_date    INTEGER,
             position    REAL NOT NULL,
             created_at  INTEGER NOT NULL,
             updated_at  INTEGER NOT NULL,
             archived_at INTEGER,
             next_action TEXT,
             blocked_reason TEXT,
             acceptance_criteria TEXT,
             claimed_by TEXT,
             claimed_at INTEGER,
             lease_expires_at INTEGER,
             handoff_note TEXT,
             last_verification TEXT,
             UNIQUE(board_id, key_text)
         );
         INSERT INTO cards_rollback
             (id, board_id, column_id, key_text, title, body, status, priority,
              assignee, due_date, position, created_at, updated_at, archived_at,
              next_action, blocked_reason, acceptance_criteria, claimed_by,
              claimed_at, lease_expires_at, handoff_note, last_verification)
         SELECT
              id, board_id, column_id, key_text, title, body, status, priority,
              assignee, due_date, position, created_at, updated_at, archived_at,
              next_action, blocked_reason, acceptance_criteria, claimed_by,
              claimed_at, lease_expires_at, handoff_note, last_verification
           FROM cards;
         DROP TABLE cards;
         ALTER TABLE cards_rollback RENAME TO cards;
         CREATE INDEX idx_cards_board_col ON cards(board_id, column_id, position);
         CREATE INDEX idx_cards_key ON cards(board_id, key_text);
         PRAGMA foreign_keys = ON;",
    )
    .unwrap();
    conn.pragma_update(None, "user_version", version).unwrap();
}

#[test]
fn migrations_are_idempotent_and_persist() {
    let db = TempDb(temp_db("persist"));
    let key;
    {
        let mut s = Store::open(&db.0).unwrap();
        let b = s.ensure_default_board().unwrap();
        key = s
            .create_card(&b.id, None, "persisted", "body", "t")
            .unwrap()
            .key;
    }
    // Reopen: migration must not re-run or error, and data must survive.
    {
        let mut s = Store::open(&db.0).unwrap();
        let b = s.ensure_default_board().unwrap();
        let card = s.card_by_key(&b.id, &key).unwrap();
        assert!(card.is_some(), "card should survive a reopen");
        assert_eq!(s.cards(&b.id).unwrap().len(), 1);
        // ensure_default_board on an existing DB must not duplicate columns.
        assert_eq!(s.columns(&b.id).unwrap().len(), BACKLOG_BOARD_COLUMNS.len());
    }
}

#[test]
fn migration_12_renames_lone_main_board_to_backlog() {
    let db = TempDb(temp_db("main-to-backlog"));
    let card_id;
    {
        let mut s = Store::open(&db.0).unwrap();
        let b = s.ensure_default_board().unwrap();
        card_id = s
            .create_card(&b.id, None, "keep me", "", "test")
            .unwrap()
            .id;
    }
    Connection::open(&db.0)
        .unwrap()
        .execute_batch(
            "UPDATE boards SET slug = 'main', name = 'Main' WHERE slug = 'backlog';
             UPDATE ui_state SET value = 'main' WHERE key = 'tui.board';",
        )
        .unwrap();
    rollback_cards_to_schema_15_and_set_user_version(&db.0, 11);

    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();

    assert_eq!(backlog.slug, "backlog");
    assert_eq!(backlog.name, "Backlog");
    assert!(s.board_by_slug("main").unwrap().is_none());
    assert!(s
        .cards(&backlog.id)
        .unwrap()
        .iter()
        .any(|c| c.id == card_id));
}

#[test]
fn migration_12_drops_main_when_backlog_already_exists() {
    let db = TempDb(temp_db("drop-main"));
    let backlog_card_id;
    {
        let mut s = Store::open(&db.0).unwrap();
        let backlog = s.ensure_default_board().unwrap();
        backlog_card_id = s
            .create_card(&backlog.id, None, "keep backlog", "", "test")
            .unwrap()
            .id;
        let main = s
            .create_board("Main", BoardColumnTemplate::Planning)
            .unwrap();
        assert_eq!(main.slug, "main");
        s.create_card(&main.id, Some("Today"), "drop main", "", "test")
            .unwrap();
    }
    rollback_cards_to_schema_15_and_set_user_version(&db.0, 11);

    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();

    assert!(s.board_by_slug("main").unwrap().is_none());
    let cards = s.cards(&backlog.id).unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].id, backlog_card_id);
}

#[test]
fn migration_13_drops_duplicate_backlog_boards() {
    let db = TempDb(temp_db("unique-backlog"));
    {
        let mut s = Store::open(&db.0).unwrap();
        s.ensure_default_board().unwrap();
    }
    Connection::open(&db.0)
        .unwrap()
        .execute_batch(
            "INSERT INTO boards
                (id, name, slug, key_prefix, card_seq, sort_order, archived_at, created_at, updated_at)
             VALUES ('dup-backlog', ' Backlog ', 'backlog-2', 'BAC', 0, 9, NULL, 1, 1);
             INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
             VALUES ('dup-backlog-col', 'dup-backlog', 'Backlog', 0, NULL, 1);",
        )
        .unwrap();
    rollback_cards_to_schema_15_and_set_user_version(&db.0, 12);

    let s = Store::open(&db.0).unwrap();

    assert!(s.board_by_slug("backlog").unwrap().is_some());
    assert!(s.board_by_slug("backlog-2").unwrap().is_none());
}

#[test]
fn migration_13_collapses_backlog_board_to_one_column() {
    let db = TempDb(temp_db("backlog-one-column"));
    {
        let mut s = Store::open(&db.0).unwrap();
        let backlog = s.ensure_default_board().unwrap();
        let backlog_col = s.columns(&backlog.id).unwrap().remove(0);
        Connection::open(&db.0)
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
                 VALUES ('today-col', '{}', 'Today', 1, NULL, 1);
                 INSERT INTO cards
                    (id, board_id, column_id, key_text, title, body, status, priority,
                     assignee, due_date, position, created_at, updated_at, archived_at)
                 VALUES
                    ('extra-card', '{}', 'today-col', 'KB-1', 'normalize me', '', 'open', 1,
                     NULL, NULL, 1.0, 1, 1, NULL);
                 UPDATE boards SET card_seq = 1 WHERE id = '{}';",
                backlog.id, backlog.id, backlog.id
            ))
            .unwrap();
        rollback_cards_to_schema_15_and_set_user_version(&db.0, 12);
        assert_ne!(backlog_col.id, "today-col");
    }

    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();
    let cols = s.columns(&backlog.id).unwrap();
    let cards = s.cards(&backlog.id).unwrap();

    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].name, "Backlog");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].column_id, cols[0].id);
}

#[test]
fn card_is_stale_after_threshold() {
    let db = TempDb(temp_db("stale-card"));
    {
        let mut s = Store::open(&db.0).unwrap();
        let b = s.ensure_default_board().unwrap();
        s.create_card(&b.id, None, "old task", "", "t").unwrap();
    }
    let old_updated_at = now_ms() - STALE_CARD_MS - 1;
    Connection::open(&db.0)
        .unwrap()
        .execute(
            "UPDATE cards SET updated_at = ?1 WHERE key_text = 'KB-1'",
            [old_updated_at],
        )
        .unwrap();

    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    let card = s.card_by_key(&b.id, "KB-1").unwrap().unwrap();
    assert!(card_is_stale(&card));
}

#[test]
fn concurrent_writers_never_collide_on_keys() {
    // Two threads, each with its own connection to the same file, hammer
    // create_card. The shared board.card_seq counter is the contention point;
    // BEGIN IMMEDIATE + busy_timeout must serialise it so every key is unique.
    let db = TempDb(temp_db("concurrent"));
    let board_id = {
        let mut s = Store::open(&db.0).unwrap();
        s.ensure_default_board().unwrap().id
    };

    const PER_THREAD: usize = 40;
    let mut handles = Vec::new();
    for t in 0..2 {
        let path = db.0.clone();
        let bid = board_id.clone();
        handles.push(std::thread::spawn(move || {
            let mut s = Store::open(&path).unwrap();
            for i in 0..PER_THREAD {
                s.create_card(&bid, None, &format!("t{t}-{i}"), "", "t")
                    .expect("create under contention should succeed");
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let s = Store::open(&db.0).unwrap();
    let cards = s.cards(&board_id).unwrap();
    assert_eq!(cards.len(), PER_THREAD * 2, "no writes lost");
    let keys: HashSet<_> = cards.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys.len(), PER_THREAD * 2, "all keys distinct");
}

#[test]
fn reorder_is_noop_at_the_edges() {
    let db = TempDb(temp_db("reorder"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    for n in ["a", "b", "c"] {
        s.create_card(&b.id, None, n, "", "t").unwrap();
    }
    let todo = s
        .columns(&b.id)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Backlog")
        .unwrap();
    let order = |s: &Store| -> Vec<String> {
        s.cards(&b.id)
            .unwrap()
            .into_iter()
            .filter(|c| c.column_id == todo.id)
            .map(|c| c.key)
            .collect()
    };

    // Top card up: unchanged.
    s.reorder_card(&b.id, "KB-1", -1).unwrap();
    assert_eq!(order(&s), vec!["KB-1", "KB-2", "KB-3"]);
    // Bottom card down: unchanged.
    s.reorder_card(&b.id, "KB-3", 1).unwrap();
    assert_eq!(order(&s), vec!["KB-1", "KB-2", "KB-3"]);
    // Middle down: swaps with KB-3.
    s.reorder_card(&b.id, "KB-2", 1).unwrap();
    assert_eq!(order(&s), vec!["KB-1", "KB-3", "KB-2"]);
}

#[test]
fn move_between_columns_lands_at_the_end() {
    let db = TempDb(temp_db("move"));
    let mut s = Store::open(&db.0).unwrap();
    s.ensure_default_board().unwrap();
    let b = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();
    s.create_card(&b.id, Some("This week"), "existing", "", "t")
        .unwrap(); // WOR-1
    s.create_card(&b.id, Some("Today"), "mover", "", "t")
        .unwrap(); // WOR-2

    s.move_card(&b.id, "WOR-2", "This week", "t").unwrap();
    let doing = s
        .columns(&b.id)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "This week")
        .unwrap();
    let order: Vec<String> = s
        .cards(&b.id)
        .unwrap()
        .into_iter()
        .filter(|c| c.column_id == doing.id)
        .map(|c| c.key)
        .collect();
    assert_eq!(
        order,
        vec!["WOR-1", "WOR-2"],
        "moved card appends after existing"
    );
}

#[test]
fn multiple_boards_are_isolated() {
    let db = TempDb(temp_db("boards"));
    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();
    assert!(s.board_by_slug("main").unwrap().is_none());
    assert_eq!(
        s.columns(&backlog.id)
            .unwrap()
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Backlog"]
    );

    let work = s
        .create_board("Work Board", BoardColumnTemplate::Planning)
        .unwrap();
    assert_eq!(work.slug, "work-board");
    assert_eq!(work.key_prefix, "WB");

    // Same name again -> a distinct, unique slug.
    let work2 = s
        .create_board("Work Board", BoardColumnTemplate::Planning)
        .unwrap();
    assert_eq!(work2.slug, "work-board-2");

    // Keys are namespaced per board and use that board's prefix.
    let a = s
        .create_card(&backlog.id, None, "on backlog", "", "t")
        .unwrap();
    let b = s
        .create_card(&work.id, Some("Today"), "on work", "", "t")
        .unwrap();
    assert_eq!(a.key, "KB-1");
    assert_eq!(b.key, "WB-1");

    // Cards don't leak across boards.
    assert_eq!(s.cards(&backlog.id).unwrap().len(), 1);
    assert_eq!(s.cards(&work.id).unwrap().len(), 1);
    assert_eq!(s.list_boards().unwrap().len(), 3);
    assert_eq!(
        s.list_boards()
            .unwrap()
            .iter()
            .map(|b| b.slug.as_str())
            .collect::<Vec<_>>(),
        vec!["backlog", "work-board", "work-board-2"]
    );

    // Boards can be reordered and the order survives reloads.
    s.reorder_board(&work2.id, -1).unwrap();
    assert_eq!(
        s.list_boards()
            .unwrap()
            .iter()
            .map(|b| b.slug.as_str())
            .collect::<Vec<_>>(),
        vec!["backlog", "work-board-2", "work-board"]
    );
    drop(s);
    let mut s = Store::open(&db.0).unwrap();
    assert_eq!(
        s.list_boards()
            .unwrap()
            .iter()
            .map(|b| b.slug.as_str())
            .collect::<Vec<_>>(),
        vec!["backlog", "work-board-2", "work-board"]
    );

    // Deleting requires archiving first (two-step guard against data loss).
    assert!(s.delete_board(&work.id).is_err());

    // Archiving hides the board from the active list but keeps its data.
    s.archive_board(&work.id).unwrap();
    assert_eq!(s.list_boards().unwrap().len(), 2);
    assert_eq!(s.list_boards_all().unwrap().len(), 3);
    let archived = s.board_by_slug("work-board").unwrap().unwrap();
    assert!(archived.archived_at.is_some());
    assert_eq!(s.cards(&work.id).unwrap().len(), 1);

    // Unarchive restores it.
    s.unarchive_board(&work.id).unwrap();
    assert_eq!(s.list_boards().unwrap().len(), 3);
    assert!(s
        .board_by_slug("work-board")
        .unwrap()
        .unwrap()
        .archived_at
        .is_none());

    // The Backlog board can never be archived.
    assert!(s.archive_board(&backlog.id).is_err());

    // Archived boards can be deleted; the cascade leaves others intact.
    s.archive_board(&work.id).unwrap();
    s.delete_board(&work.id).unwrap();
    assert_eq!(s.list_boards().unwrap().len(), 2);
    assert!(s.board_by_slug("work-board").unwrap().is_none());
    assert_eq!(s.cards(&backlog.id).unwrap().len(), 1);
}

#[test]
fn recent_labels_reflect_usage() {
    let db = TempDb(temp_db("recent"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "c", "", "t").unwrap();
    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            add_labels: Some(vec!["bug".into(), "ui".into()]),
            ..Default::default()
        },
        "t",
    )
    .unwrap();

    // Just-used labels appear within a generous window...
    let recent: Vec<String> = s
        .recent_labels(30 * 24 * 60 * 60 * 1000)
        .unwrap()
        .into_iter()
        .map(|l| l.name)
        .collect();
    assert!(recent.contains(&"bug".to_string()));
    assert!(recent.contains(&"ui".to_string()));

    // ...and are excluded by an impossibly tight (negative) window.
    assert!(s.recent_labels(-1).unwrap().is_empty());
}

#[test]
fn column_crud_and_delete_relocates_cards() {
    let db = TempDb(temp_db("columns"));
    let mut s = Store::open(&db.0).unwrap();
    s.ensure_default_board().unwrap();
    let b = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();
    let names = |s: &Store| -> Vec<String> {
        s.columns(&b.id)
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect()
    };
    assert_eq!(names(&s), ["Backlog", "Today", "This week", "This month"]);

    // Add + rename.
    let extra = s.add_column(&b.id, "保留").unwrap();
    assert_eq!(names(&s).last().unwrap(), "保留");
    assert!(
        s.add_column(&b.id, "Today").is_err(),
        "duplicate name rejected"
    );
    s.rename_column(&extra.id, "アイスボックス").unwrap();
    assert!(names(&s).contains(&"アイスボックス".to_string()));

    // Reorder: move the last column left once.
    s.reorder_column(&b.id, &extra.id, -1).unwrap();
    assert_eq!(names(&s)[3], "アイスボックス");

    // Put a card in This week, then delete that column moving cards to Today.
    let cols = s.columns(&b.id).unwrap();
    let doing = cols.iter().find(|c| c.name == "This week").unwrap().clone();
    let todo = cols.iter().find(|c| c.name == "Today").unwrap().clone();
    s.create_card(&b.id, Some("This week"), "in progress", "", "t")
        .unwrap();

    assert!(
        s.delete_column(&b.id, &doing.id, &doing.id).is_err(),
        "self-move rejected"
    );
    s.delete_column(&b.id, &doing.id, &todo.id).unwrap();
    assert!(!names(&s).contains(&"This week".to_string()));
    // The card survived and now lives in Today.
    let card = s.card_by_key(&b.id, "WOR-1").unwrap().unwrap();
    assert_eq!(card.column_id, todo.id);
}

#[test]
fn cannot_delete_the_last_column() {
    let db = TempDb(temp_db("lastcol"));
    let mut s = Store::open(&db.0).unwrap();
    s.ensure_default_board().unwrap();
    let b = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();
    let cols = s.columns(&b.id).unwrap();
    // Delete down to one, then the final delete must fail.
    let keep = cols[0].clone();
    for c in cols.iter().skip(1) {
        s.delete_column(&b.id, &c.id, &keep.id).unwrap();
    }
    assert_eq!(s.columns(&b.id).unwrap().len(), 1);
    let other = s.add_column(&b.id, "tmp").unwrap();
    s.delete_column(&b.id, &other.id, &keep.id).unwrap();
    assert!(s.delete_column(&b.id, &keep.id, &keep.id).is_err());
}

#[test]
fn data_version_detects_other_connections() {
    let db = TempDb(temp_db("dataver"));
    let mut writer = Store::open(&db.0).unwrap();
    let b = writer.ensure_default_board().unwrap();

    let reader = Store::open(&db.0).unwrap();
    let v0 = reader.data_version().unwrap();

    // A write on a *different* connection bumps the reader's data_version.
    writer
        .create_card(&b.id, None, "external", "", "agent")
        .unwrap();
    let v1 = reader.data_version().unwrap();
    assert_ne!(v0, v1, "external commit must change data_version");

    // No further writes -> stable (no spurious reloads).
    assert_eq!(reader.data_version().unwrap(), v1);
}

#[test]
fn unknown_card_and_column_error_cleanly() {
    let db = TempDb(temp_db("errors"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    assert!(s
        .update_card(&b.id, "KB-999", &CardPatch::default(), "t")
        .is_err());
    assert!(s.create_card(&b.id, Some("Nope"), "x", "", "t").is_err());
    assert!(s.move_card(&b.id, "KB-999", "This month", "t").is_err());
}

#[test]
fn update_card_rejects_stale_updated_at() {
    let db = TempDb(temp_db("stale"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();

    s.create_card(&b.id, None, "task", "", "t").unwrap();
    let stale = s.card_by_key(&b.id, "KB-1").unwrap().unwrap().updated_at;

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            title: Some("first edit".into()),
            ..Default::default()
        },
        "t",
    )
    .unwrap();

    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                title: Some("stale edit".into()),
                expected_updated_at: Some(stale),
                ..Default::default()
            },
            "t",
        )
        .is_err());
}

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

#[test]
fn card_agent_fields_update_and_clear() {
    let db = TempDb(temp_db("agent-fields"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "agent task", "", "t").unwrap();

    let patched = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                next_action: Some("run cargo test".into()),
                blocked_reason: Some("waiting for schema decision".into()),
                acceptance_criteria: Some("MCP can update and read fields".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert_eq!(patched.next_action.as_deref(), Some("run cargo test"));
    assert_eq!(
        patched.blocked_reason.as_deref(),
        Some("waiting for schema decision")
    );
    assert_eq!(
        patched.acceptance_criteria.as_deref(),
        Some("MCP can update and read fields")
    );
    let json = s.export_json(&b.id).unwrap();
    assert!(json.contains("\"next_action\": \"run cargo test\""));
    assert!(json.contains("\"blocked_reason\": \"waiting for schema decision\""));
    assert!(json.contains("\"acceptance_criteria\": \"MCP can update and read fields\""));

    let cleared = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                blocked_reason: Some("".into()),
                next_action: Some("  write release note  ".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert!(cleared.blocked_reason.is_none());
    assert_eq!(cleared.next_action.as_deref(), Some("write release note"));
}

#[test]
fn board_agent_context_persists_and_exports() {
    let db = TempDb(temp_db("board-agent-context"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s
        .create_board("Repo Work", BoardColumnTemplate::Workflow)
        .unwrap();

    s.update_board_agent_context(
        &b.id,
        Some("Run cargo test -p kanban-core before closing cards."),
    )
    .unwrap();

    let json = s.export_json(&b.id).unwrap();
    assert!(
        json.contains("\"agent_context\": \"Run cargo test -p kanban-core before closing cards.\"")
    );
    let md = s.export_markdown(&b.id).unwrap();
    assert!(md.contains("## Board Agent Context"));
    assert!(md.contains("Run cargo test -p kanban-core before closing cards."));

    drop(s);
    let s = Store::open(&db.0).unwrap();
    let persisted = s.board_by_slug("repo-work").unwrap().unwrap();
    assert_eq!(
        persisted.agent_context.as_deref(),
        Some("Run cargo test -p kanban-core before closing cards.")
    );
}

#[test]
fn card_agent_execution_metadata_update_clear_validate_and_persist() {
    let db = TempDb(temp_db("agent-execution-metadata"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "agent execution", "", "t")
        .unwrap();

    let patched = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                agent_weight: Some(Some(3)),
                agent_effort: Some("high-reasoning".into()),
                suggested_model: Some("gpt-5".into()),
                expected_tokens: Some(Some(12_000)),
                human_intervention: Some("review".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert_eq!(patched.agent_weight, Some(3));
    assert_eq!(patched.agent_effort.as_deref(), Some("high-reasoning"));
    assert_eq!(patched.suggested_model.as_deref(), Some("gpt-5"));
    assert_eq!(patched.expected_tokens, Some(12_000));
    assert_eq!(patched.human_intervention.as_deref(), Some("review"));

    let json = s.export_json(&b.id).unwrap();
    assert!(json.contains("\"agent_weight\": 3"));
    assert!(json.contains("\"agent_effort\": \"high-reasoning\""));
    assert!(json.contains("\"suggested_model\": \"gpt-5\""));
    assert!(json.contains("\"expected_tokens\": 12000"));
    assert!(json.contains("\"human_intervention\": \"review\""));

    drop(s);
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    let persisted = s.card_by_key(&b.id, "KB-1").unwrap().unwrap();
    assert_eq!(persisted.agent_weight, Some(3));
    assert_eq!(persisted.expected_tokens, Some(12_000));

    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                agent_weight: Some(Some(0)),
                ..Default::default()
            },
            "t",
        )
        .is_err());
    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                expected_tokens: Some(Some(0)),
                ..Default::default()
            },
            "t",
        )
        .is_err());
    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                human_intervention: Some("maybe".into()),
                ..Default::default()
            },
            "t",
        )
        .is_err());

    let cleared = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                agent_weight: Some(None),
                agent_effort: Some(" ".into()),
                suggested_model: Some("".into()),
                expected_tokens: Some(None),
                human_intervention: Some("".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert_eq!(cleared.agent_weight, None);
    assert_eq!(cleared.agent_effort, None);
    assert_eq!(cleared.suggested_model, None);
    assert_eq!(cleared.expected_tokens, None);
    assert_eq!(cleared.human_intervention, None);
}

#[test]
fn agent_registration_token_lifetime_defaults_and_caps() {
    let db = TempDb(temp_db("agent-token-lifetime"));
    let mut s = Store::open(&db.0).unwrap();

    let defaulted = s.register_agent("codex", None, None, None).unwrap();
    let default_lifetime = defaulted.registration.expires_at - defaulted.registration.registered_at;
    assert!(
        (119 * 60_000..=120 * 60_000).contains(&default_lifetime),
        "default lifetime was {default_lifetime}"
    );

    let capped = s
        .register_agent("claude", None, None, Some(7 * 24 * 60))
        .unwrap();
    let capped_lifetime = capped.registration.expires_at - capped.registration.registered_at;
    assert!(
        (23 * 60 * 60_000..=24 * 60 * 60_000).contains(&capped_lifetime),
        "capped lifetime was {capped_lifetime}"
    );
}

#[test]
fn card_claims_conflict_expire_and_release() {
    let db = TempDb(temp_db("card-claims"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "claim task", "", "t").unwrap();
    let codex = s.register_agent("codex", None, None, None).unwrap();
    let claude = s.register_agent("claude", None, None, None).unwrap();

    let claimed = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(codex.registration.assigned_identity.clone()),
                claim_token: Some(codex.claim_token.clone()),
                lease_minutes: Some(30),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(
        claimed.claimed_by.as_deref(),
        Some(codex.registration.assigned_identity.as_str())
    );
    assert!(claimed.claimed_at.is_some());
    assert!(claimed.lease_expires_at.unwrap() > claimed.claimed_at.unwrap());

    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(claude.registration.assigned_identity.clone()),
                claim_token: Some(claude.claim_token.clone()),
                ..Default::default()
            },
            "agent",
        )
        .is_err());

    let renewed = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(codex.registration.assigned_identity.clone()),
                claim_token: Some(codex.claim_token.clone()),
                lease_minutes: Some(120),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(
        renewed.claimed_by.as_deref(),
        Some(codex.registration.assigned_identity.as_str())
    );

    {
        let conn = Connection::open(&db.0).unwrap();
        conn.execute(
            "UPDATE cards SET lease_expires_at = 1 WHERE key_text = 'KB-1'",
            [],
        )
        .unwrap();
    }
    let taken_over = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(claude.registration.assigned_identity.clone()),
                claim_token: Some(claude.claim_token.clone()),
                lease_minutes: Some(10),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(
        taken_over.claimed_by.as_deref(),
        Some(claude.registration.assigned_identity.as_str())
    );

    let released = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                release_claim: Some(true),
                claim_token: Some(claude.claim_token),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert!(released.claimed_by.is_none());
    assert!(released.claimed_at.is_none());
    assert!(released.lease_expires_at.is_none());
}

#[test]
fn archiving_card_clears_claim() {
    let db = TempDb(temp_db("archive-clears-claim"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    let agent = s.register_agent("codex", None, None, None).unwrap();
    s.create_card(&b.id, None, "finish task", "", "t").unwrap();

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            claim: Some(agent.registration.assigned_identity),
            claim_token: Some(agent.claim_token),
            ..Default::default()
        },
        "agent",
    )
    .unwrap();

    let archived = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                archived: Some(true),
                agent_state: Some("done".into()),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(archived.agent_state, "done");
    assert!(archived.archived_at.is_some());
    assert!(archived.claimed_by.is_none());
    assert!(archived.claimed_at.is_none());
    assert!(archived.lease_expires_at.is_none());
}

#[test]
fn card_search_uses_fts_index_and_stays_in_sync() {
    let db = TempDb(temp_db("card-search"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "fix parser", "spreadsheet import", "t")
        .unwrap();
    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            add_labels: Some(vec!["regression".into()]),
            next_action: Some("write fts coverage".into()),
            ..Default::default()
        },
        "t",
    )
    .unwrap();

    assert_eq!(s.search_cards(&b.id, "parser").unwrap()[0].key, "KB-1");
    assert_eq!(s.search_cards(&b.id, "regression").unwrap()[0].key, "KB-1");
    assert_eq!(
        s.search_cards(&b.id, "fts coverage").unwrap()[0].key,
        "KB-1"
    );

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            remove_labels: Some(vec!["regression".into()]),
            ..Default::default()
        },
        "t",
    )
    .unwrap();
    assert!(s.search_cards(&b.id, "regression").unwrap().is_empty());

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            archived: Some(true),
            ..Default::default()
        },
        "t",
    )
    .unwrap();
    assert!(s.search_cards(&b.id, "parser").unwrap().is_empty());
}

#[test]
fn move_card_between_boards_with_update_card() {
    let db = TempDb(temp_db("move-boards"));
    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();
    let work = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();

    let card = s
        .create_card(&backlog.id, None, "handoff", "", "test")
        .unwrap();
    assert_eq!(card.key, "KB-1");

    let moved = s
        .update_card(
            &backlog.id,
            &card.key,
            &CardPatch {
                move_to_board: Some(work.id.clone()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    assert_eq!(moved.board_id, work.id);
    assert_ne!(moved.key, card.key);
    assert!(s.card_by_key(&backlog.id, &card.key).unwrap().is_none());

    let work_cols = s.columns(&work.id).unwrap();
    let work_backlog = work_cols
        .into_iter()
        .find(|c| c.name == "Backlog")
        .expect("backlog column on work");
    let current = s.card_by_key(&work.id, &moved.key).unwrap().unwrap();
    assert_eq!(current.id, card.id);
    assert_eq!(current.column_id, work_backlog.id);
    assert_eq!(s.cards(&backlog.id).unwrap().len(), 0);
    assert_eq!(s.cards(&work.id).unwrap().len(), 1);

    let activity = s.card_activity(&current.id, 10).unwrap();
    let move_log = activity
        .iter()
        .find(|log| log.action == "move_board")
        .expect("move_board activity");
    let payload: serde_json::Value = serde_json::from_str(&move_log.payload_json).unwrap();
    assert_eq!(payload["old_key"], "KB-1");
    assert_eq!(payload["new_key"], moved.key);
    assert_eq!(payload["source_board"]["slug"], "backlog");
    assert_eq!(payload["destination_board"]["slug"], "work");
}

#[test]
fn dependencies_validate_graph_and_drive_readiness() {
    let db = TempDb(temp_db("dependencies-readiness"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    for title in ["A", "B", "C", "D", "E"] {
        s.create_card(&b.id, None, title, "", "t").unwrap();
    }

    let b_deps = s
        .set_card_dependencies(&b.id, "KB-2", &["KB-1".to_string()], "agent")
        .unwrap();
    assert_eq!(b_deps[0].downstream_key, "KB-2");
    assert_eq!(b_deps[0].upstream_key, "KB-1");
    s.set_card_dependencies(&b.id, "KB-3", &["KB-1".to_string()], "agent")
        .unwrap();
    s.set_card_dependencies(&b.id, "KB-4", &["KB-1".to_string()], "agent")
        .unwrap();
    s.set_card_dependencies(
        &b.id,
        "KB-5",
        &["KB-2".to_string(), "KB-3".to_string(), "KB-4".to_string()],
        "agent",
    )
    .unwrap();

    let all = s.card_dependencies(&b.id).unwrap();
    assert_eq!(all.len(), 6);
    assert!(s.card_readiness(&b.id, "KB-1").unwrap().ready);
    let blocked_b = s.card_readiness(&b.id, "KB-2").unwrap();
    assert!(!blocked_b.ready);
    assert_eq!(blocked_b.blocked_by[0].key, "KB-1");

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            agent_state: Some("done".into()),
            archived: Some(true),
            ..Default::default()
        },
        "agent",
    )
    .unwrap();
    assert!(s.card_readiness(&b.id, "KB-2").unwrap().ready);
    assert!(s.card_readiness(&b.id, "KB-3").unwrap().ready);
    assert!(s.card_readiness(&b.id, "KB-4").unwrap().ready);
    assert!(!s.card_readiness(&b.id, "KB-5").unwrap().ready);

    for key in ["KB-2", "KB-3", "KB-4"] {
        s.update_card(
            &b.id,
            key,
            &CardPatch {
                agent_state: Some("done".into()),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    }
    assert!(s.card_readiness(&b.id, "KB-5").unwrap().ready);

    let cycle = s.set_card_dependencies(&b.id, "KB-1", &["KB-5".to_string()], "agent");
    assert!(cycle.unwrap_err().to_string().contains("cycle"));

    let missing = s.set_card_dependencies(&b.id, "KB-5", &["KB-999".to_string()], "agent");
    assert!(missing.unwrap_err().to_string().contains("missing card"));
}

#[test]
fn create_cards_from_plan_rolls_back_dependency_failures() {
    let db = TempDb(temp_db("plan-import-rollback"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "existing upstream", "", "t")
        .unwrap();

    let err = s
        .create_cards_from_plan(
            &b.id,
            &[
                CardCreateDraft {
                    alias: Some("A".into()),
                    title: "A".into(),
                    depends_on: vec!["B".into()],
                    ..Default::default()
                },
                CardCreateDraft {
                    alias: Some("B".into()),
                    title: "B".into(),
                    depends_on: vec!["KB-1".into()],
                    ..Default::default()
                },
            ],
            "agent",
        )
        .unwrap();
    assert_eq!(err[0].key, "KB-2");
    assert_eq!(err[1].key, "KB-3");

    let cycle = s
        .create_cards_from_plan(
            &b.id,
            &[CardCreateDraft {
                alias: Some("C".into()),
                title: "C".into(),
                depends_on: vec!["KB-4".into()],
                ..Default::default()
            }],
            "agent",
        )
        .unwrap_err();
    assert!(cycle.to_string().contains("cannot depend on itself"));

    let cards = s.cards(&b.id).unwrap();
    assert_eq!(cards.len(), 3);
    assert!(s.card_by_key(&b.id, "KB-4").unwrap().is_none());
}

#[test]
fn create_cards_from_plan_rejects_alias_colliding_with_existing_key() {
    let db = TempDb(temp_db("plan-import-alias-key-collision"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "existing", "", "t").unwrap();

    let err = s
        .create_cards_from_plan(
            &b.id,
            &[CardCreateDraft {
                alias: Some("KB-1".into()),
                title: "new work".into(),
                ..Default::default()
            }],
            "agent",
        )
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("alias conflicts with existing card key 'KB-1'"));
    assert_eq!(s.cards(&b.id).unwrap().len(), 1);
    assert!(s.card_by_key(&b.id, "KB-2").unwrap().is_none());
}

#[test]
fn archived_unfinished_dependency_blocks_downstream() {
    let db = TempDb(temp_db("dependencies-archived-unfinished"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "upstream", "", "t").unwrap();
    s.create_card(&b.id, None, "downstream", "", "t").unwrap();
    s.set_card_dependencies(&b.id, "KB-2", &["KB-1".to_string()], "agent")
        .unwrap();

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            archived: Some(true),
            ..Default::default()
        },
        "agent",
    )
    .unwrap();

    let readiness = s.card_readiness(&b.id, "KB-2").unwrap();
    assert!(!readiness.ready);
    assert_eq!(readiness.blocked_by[0].key, "KB-1");
    assert!(readiness.blocked_by[0].archived_at.is_some());
}
