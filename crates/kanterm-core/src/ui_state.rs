use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::Store;

impl Store {
    /// A counter that changes whenever **another** connection commits to this
    /// database (SQLite's `PRAGMA data_version`). It is *not* affected by writes
    /// on this same connection, which makes it a cheap "did an agent change the
    /// board?" signal for the TUI to poll. O(1), no table scan.
    pub fn data_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .pragma_query_value(None, "data_version", |r| r.get(0))?)
    }

    pub fn get_ui_state(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM ui_state WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_ui_state(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO ui_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;

    #[test]
    fn ui_state_roundtrips_and_overwrites() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.get_ui_state("tui.focus").unwrap(), None);

        store.set_ui_state("tui.focus", "1").unwrap();
        assert_eq!(
            store.get_ui_state("tui.focus").unwrap().as_deref(),
            Some("1")
        );

        store.set_ui_state("tui.focus", "2").unwrap();
        assert_eq!(
            store.get_ui_state("tui.focus").unwrap().as_deref(),
            Some("2")
        );
    }
}
