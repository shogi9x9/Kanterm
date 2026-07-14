use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use std::path::{Path, PathBuf};

use crate::SCHEMA_VERSION;

/// Ordered list of migrations. Index 0 takes the DB from user_version 0 -> 1.
/// We deliberately avoid a migration framework: a plain array + integer compare
/// is enough and adds no dependencies (per design review).
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_init.sql"),
    include_str!("migrations/0002_label_last_used.sql"),
    include_str!("migrations/0003_board_archive.sql"),
    include_str!("migrations/0004_memories.sql"),
    include_str!("migrations/0005_memory_retention.sql"),
    include_str!("migrations/0006_card_agent_fields.sql"),
    include_str!("migrations/0007_card_claims.sql"),
    include_str!("migrations/0008_board_order.sql"),
    include_str!("migrations/0009_main_planning_columns.sql"),
    include_str!("migrations/0010_card_agent_notes.sql"),
    include_str!("migrations/0011_backlog_default_board_name.sql"),
    include_str!("migrations/0012_backlog_protected_board.sql"),
    include_str!("migrations/0013_unique_backlog_board.sql"),
    include_str!("migrations/0014_agent_registrations.sql"),
    include_str!("migrations/0015_card_search_fts.sql"),
    include_str!("migrations/0016_agent_execution_metadata.sql"),
    include_str!("migrations/0017_card_dependencies.sql"),
    include_str!("migrations/0018_board_agent_context.sql"),
    include_str!("migrations/0019_agent_handoffs.sql"),
    include_str!("migrations/0020_handoff_results.sql"),
];

pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    /// Default DB path: ~/.local/share/kanban/kanban.db (XDG data dir).
    pub fn default_db_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("dev", "kanban", "kanban")
            .ok_or_else(|| anyhow!("could not determine a home directory"))?;
        Ok(dirs.data_dir().join("kanban.db"))
    }

    /// Open (creating if needed) the database at `path`, applying pragmas and
    /// migrations. This is the only place a connection is configured.
    pub fn open(path: impl AsRef<Path>) -> Result<Store> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating data dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        Self::configure(&conn)?;
        let mut store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    /// In-memory store for tests.
    pub fn open_in_memory() -> Result<Store> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        let mut store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn database_schema_version(path: impl AsRef<Path>) -> Result<i64> {
        let path = path.as_ref();
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_context(|| format!("opening database {}", path.display()))?;
        conn.pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(Into::into)
    }

    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<()> {
        let destination = destination.as_ref();
        if destination.exists() {
            return Err(anyhow!(
                "backup destination already exists: {}",
                destination.display()
            ));
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating backup dir {}", parent.display()))?;
        }
        self.conn
            .execute(
                "VACUUM INTO ?1",
                params![destination.to_string_lossy().as_ref()],
            )
            .with_context(|| format!("writing backup {}", destination.display()))?;
        Ok(())
    }

    fn configure(conn: &Connection) -> Result<()> {
        // WAL lets readers and a single writer coexist; the rest make
        // concurrent TUI + MCP access safe. foreign_keys defaults OFF in SQLite.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        Ok(())
    }

    fn migrate(&mut self) -> Result<()> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            // A newer binary created this DB. Refuse to touch it rather than
            // silently corrupting data with stale write logic.
            return Err(anyhow!(
                "database schema version {version} is newer than this build supports ({SCHEMA_VERSION}); please update kanban"
            ));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let target = (i + 1) as i64;
            if version < target {
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                tx.execute_batch(sql)
                    .with_context(|| format!("applying migration {target}"))?;
                tx.pragma_update(None, "user_version", target)?;
                tx.commit()?;
            }
        }
        self.run_due_memory_gc()?;
        Ok(())
    }

    /// Guard every write path: an MCP server that has been running across an
    /// upgrade must not write with stale logic.
    pub(crate) fn assert_writable(&self) -> Result<()> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version != SCHEMA_VERSION {
            return Err(anyhow!(
                "database schema changed under this process (db={version}, build={SCHEMA_VERSION}); restart kanban"
            ));
        }
        Ok(())
    }
}
