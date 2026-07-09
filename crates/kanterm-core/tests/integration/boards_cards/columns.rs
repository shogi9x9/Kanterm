use crate::common::{temp_db, TempDb};
use kanterm_core::{BoardColumnTemplate, CardPatch, Store};

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
