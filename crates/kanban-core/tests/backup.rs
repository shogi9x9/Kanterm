mod common;

use common::{temp_db, TempDb};
use kanban_core::{Store, SCHEMA_VERSION};

#[test]
fn backup_to_writes_restorable_sqlite_database() {
    let db = TempDb(temp_db("backup-source"));
    let backup = temp_db("backup-copy");
    if backup.exists() {
        std::fs::remove_file(&backup).unwrap();
    }
    let mut s = Store::open(&db.0).unwrap();
    let board = s.ensure_default_board().unwrap();
    s.create_card(&board.id, None, "backup me", "", "test")
        .unwrap();

    s.backup_to(&backup).unwrap();
    assert_eq!(
        Store::database_schema_version(&backup).unwrap(),
        SCHEMA_VERSION
    );

    let mut restored = Store::open(&backup).unwrap();
    let board = restored.ensure_default_board().unwrap();
    assert_eq!(restored.cards(&board.id).unwrap()[0].title, "backup me");
}
