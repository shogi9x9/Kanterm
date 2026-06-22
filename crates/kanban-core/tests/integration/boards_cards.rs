use crate::common::{temp_db, TempDb};
use kanban_core::{card_is_stale, now_ms, BoardColumnTemplate, CardPatch, Store, STALE_CARD_MS};
use rusqlite::Connection;
use std::collections::HashSet;

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
