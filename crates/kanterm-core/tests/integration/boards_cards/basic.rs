use crate::common::{temp_db, TempDb};
use kanterm_core::{card_is_stale, now_ms, BoardColumnTemplate, Store, STALE_CARD_MS};
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
