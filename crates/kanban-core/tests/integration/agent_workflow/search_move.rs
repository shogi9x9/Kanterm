use crate::common::{temp_db, TempDb};
use kanban_core::{BoardColumnTemplate, CardPatch, Store};

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
