use crate::common::{temp_db, TempDb};
use kanban_core::{BoardColumnTemplate, CardPatch, Store};

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
