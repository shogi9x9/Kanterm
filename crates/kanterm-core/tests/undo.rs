mod common;

use common::{temp_db, TempDb};
use kanterm_core::{BoardColumnTemplate, CardPatch, Store};

#[test]
fn undo_last_card_update_restores_archived_card() {
    let db = TempDb(temp_db("undo-archive"));
    let mut s = Store::open(&db.0).unwrap();
    let board = s.ensure_default_board().unwrap();
    s.create_card(&board.id, None, "accidental archive", "", "test")
        .unwrap();

    s.update_card(
        &board.id,
        "KB-1",
        &CardPatch {
            archived: Some(true),
            ..Default::default()
        },
        "test",
    )
    .unwrap();
    assert!(s.cards(&board.id).unwrap().is_empty());

    let restored = s
        .undo_last_card_update(&board.id, "test")
        .unwrap()
        .expect("undo should restore the archived card");
    assert_eq!(restored.key, "KB-1");
    assert_eq!(restored.title, "accidental archive");
    assert!(restored.archived_at.is_none());
    assert_eq!(s.cards(&board.id).unwrap().len(), 1);

    assert!(s
        .undo_last_card_update(&board.id, "test")
        .unwrap()
        .is_none());
}

#[test]
fn undo_last_card_update_restores_cross_board_move() {
    let db = TempDb(temp_db("undo-board-move"));
    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();
    let work = s
        .create_board("Work", BoardColumnTemplate::Workflow)
        .unwrap();
    s.create_card(&backlog.id, None, "wrong board", "", "test")
        .unwrap();

    let moved = s
        .update_card(
            &backlog.id,
            "KB-1",
            &CardPatch {
                move_to_board: Some(work.id.clone()),
                column: Some("Testing".into()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    assert_eq!(moved.board_id, work.id);
    assert_eq!(moved.key, "WOR-1");
    assert!(s.card_by_key(&backlog.id, "KB-1").unwrap().is_none());

    let restored = s
        .undo_last_card_update(&work.id, "test")
        .unwrap()
        .expect("undo should restore the moved card");
    assert_eq!(restored.board_id, backlog.id);
    assert_eq!(restored.key, "KB-1");
    assert_eq!(restored.title, "wrong board");
    assert!(s.card_by_key(&work.id, "WOR-1").unwrap().is_none());
    assert!(s.card_by_key(&backlog.id, "KB-1").unwrap().is_some());
}
