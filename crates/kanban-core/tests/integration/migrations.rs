use crate::common::{temp_db, TempDb};
use kanban_core::{BoardColumnTemplate, Store, BACKLOG_BOARD_COLUMNS};
use rusqlite::Connection;

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
