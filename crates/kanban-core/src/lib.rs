//! kanban-core: the single source of truth for the kanban board.
//!
//! Both the TUI (`kanban-tui`) and the MCP server (`kanban-mcp`) depend on this
//! crate and nothing else touches the database directly. All schema rules,
//! migrations, and write logic live here so the two frontends can never drift.

mod activity;
mod agents;
mod boards;
mod cards;
mod columns;
mod database;
mod dates;
mod dependencies;
mod domain;
mod export;
mod id;
mod labels;
mod memories;
mod naming;
mod position;
mod rows;
mod search;
mod text;
mod ui_state;

use anyhow::Result;
use rows::CARD_COLUMNS;

pub use database::Store;
pub(crate) use dates::MS_PER_DAY;
pub use dates::{format_date, now_ms, parse_date, today_start_ms};
pub use domain::{
    card_is_stale, priority_badge, priority_label, ActivityLog, AgentRegistration,
    AgentRegistrationResult, Board, Card, CardCreateDraft, CardDependency, CardPatch,
    CardReadiness, Column, DependencyBlockedCard, DependencyBlocker, DependencyStagePlan, Label,
    Memory, MemoryPatch, PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_NORMAL, STALE_CARD_MS,
};

/// Bump this whenever a migration is added. Stored in SQLite `PRAGMA user_version`.
pub const SCHEMA_VERSION: i64 = 18;

pub const BACKLOG_BOARD_COLUMNS: &[&str] = &["Backlog"];
pub const PROTECTED_BOARD_SLUG: &str = "backlog";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardColumnTemplate {
    Planning,
    Workflow,
    Simple,
}

impl BoardColumnTemplate {
    pub const ALL: &[BoardColumnTemplate] = &[
        BoardColumnTemplate::Planning,
        BoardColumnTemplate::Workflow,
        BoardColumnTemplate::Simple,
    ];

    pub const DEFAULT_PROJECT: BoardColumnTemplate = BoardColumnTemplate::Workflow;

    pub fn key(self) -> &'static str {
        match self {
            BoardColumnTemplate::Planning => "planning",
            BoardColumnTemplate::Workflow => "workflow",
            BoardColumnTemplate::Simple => "simple",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BoardColumnTemplate::Planning => "Planning",
            BoardColumnTemplate::Workflow => "Workflow",
            BoardColumnTemplate::Simple => "Simple",
        }
    }

    pub fn columns(self) -> &'static [&'static str] {
        match self {
            BoardColumnTemplate::Planning => &["Backlog", "Today", "This week", "This month"],
            BoardColumnTemplate::Workflow => {
                &["Todo", "In progress", "Testing", "Waiting for release"]
            }
            BoardColumnTemplate::Simple => &["Todo", "Doing", "Done"],
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.key() == key)
    }

    pub fn default_index() -> usize {
        Self::ALL
            .iter()
            .position(|t| *t == Self::DEFAULT_PROJECT)
            .unwrap_or(0)
    }
}

impl Store {
    // -- Export -------------------------------------------------------------

    /// Pretty JSON snapshot of a board (columns -> cards -> labels).
    pub fn export_json(&self, board_id: &str) -> Result<String> {
        export::export_json(self, board_id)
    }

    /// Markdown snapshot of a board, suitable for committing to git.
    pub fn export_markdown(&self, board_id: &str) -> Result<String> {
        export::export_markdown(self, board_id)
    }
}

// ---------------------------------------------------------------------------
// Row mappers & helpers
// ---------------------------------------------------------------------------

// Keep CARD_COLUMNS referenced so a future refactor notices the duplication.
#[allow(dead_code)]
const _: &str = CARD_COLUMNS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_move_and_list() {
        let mut s = Store::open_in_memory().unwrap();
        s.ensure_default_board().unwrap();
        let b = s
            .create_board("Work", BoardColumnTemplate::Planning)
            .unwrap();
        let cols = s.columns(&b.id).unwrap();
        assert_eq!(cols.len(), BoardColumnTemplate::Planning.columns().len());

        let c = s.create_card(&b.id, None, "first", "body", "test").unwrap();
        assert_eq!(c.key, "WOR-1");
        assert_eq!(c.agent_state, "open");

        let c2 = s
            .create_card(&b.id, Some("This week"), "second", "", "test")
            .unwrap();
        assert_eq!(c2.key, "WOR-2");

        let moved = s.move_card(&b.id, "WOR-1", "This month", "test").unwrap();
        let done = cols.iter().find(|c| c.name == "This month").unwrap();
        assert_eq!(moved.column_id, done.id);

        let all = s.cards(&b.id).unwrap();
        assert_eq!(all.len(), 2);

        let patched = s
            .update_card(
                &b.id,
                "WOR-2",
                &CardPatch {
                    title: Some("renamed".into()),
                    priority: Some(PRIORITY_HIGH),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();
        assert_eq!(patched.title, "renamed");
        assert_eq!(patched.priority, PRIORITY_HIGH);

        s.update_card(
            &b.id,
            "WOR-1",
            &CardPatch {
                archived: Some(true),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
        assert_eq!(s.cards(&b.id).unwrap().len(), 1);
    }

    #[test]
    fn labels_reorder_and_export() {
        let mut s = Store::open_in_memory().unwrap();
        s.ensure_default_board().unwrap();
        let b = s
            .create_board("Work", BoardColumnTemplate::Planning)
            .unwrap();

        s.create_card(&b.id, Some("Today"), "a", "", "t").unwrap();
        s.create_card(&b.id, Some("Today"), "b", "", "t").unwrap();
        s.create_card(&b.id, Some("Today"), "c", "", "t").unwrap();

        // Attach two labels to WOR-1, then drop one.
        s.update_card(
            &b.id,
            "WOR-1",
            &CardPatch {
                add_labels: Some(vec!["bug".into(), "urgent".into()]),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
        s.update_card(
            &b.id,
            "WOR-1",
            &CardPatch {
                remove_labels: Some(vec!["urgent".into()]),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
        let by_card = s.labels_by_card(&b.id).unwrap();
        let kb1 = s.card_by_key(&b.id, "WOR-1").unwrap().unwrap();
        assert_eq!(by_card.get(&kb1.id).unwrap().len(), 1);
        assert_eq!(by_card.get(&kb1.id).unwrap()[0].name, "bug");

        // Order in Todo is a, b, c. Move WOR-1 down -> b, a, c.
        s.reorder_card(&b.id, "WOR-1", 1).unwrap();
        let todo_col = s
            .columns(&b.id)
            .unwrap()
            .into_iter()
            .find(|c| c.name == "Today")
            .unwrap();
        let order: Vec<String> = s
            .cards(&b.id)
            .unwrap()
            .into_iter()
            .filter(|c| c.column_id == todo_col.id)
            .map(|c| c.key)
            .collect();
        assert_eq!(order, vec!["WOR-2", "WOR-1", "WOR-3"]);

        let json = s.export_json(&b.id).unwrap();
        assert!(json.contains("\"bug\""));
        let md = s.export_markdown(&b.id).unwrap();
        assert!(md.contains("## Today (3)"));
    }

    #[test]
    fn dates() {
        // Known anchor: 2000-01-01 is 10957 days after the epoch.
        assert_eq!(parse_date("2000-01-01").unwrap(), 10957 * MS_PER_DAY);
        for s in ["1970-01-01", "2024-02-29", "2026-06-11", "1999-12-31"] {
            assert_eq!(format_date(parse_date(s).unwrap()), s);
        }
        assert!(parse_date("2026-13-01").is_err());
        assert!(parse_date("nope").is_err());
        // Calendar-aware day validation.
        assert!(parse_date("2026-02-31").is_err());
        assert!(parse_date("2025-02-29").is_err()); // 2025 is not a leap year
        assert!(parse_date("2024-02-29").is_ok()); // 2024 is
        assert!(parse_date("2026-04-31").is_err()); // April has 30 days
        assert!(parse_date("2026-00-10").is_err());

        let mut store = Store::open_in_memory().unwrap();
        let b = store.ensure_default_board().unwrap();
        store.create_card(&b.id, None, "task", "", "t").unwrap();
        store
            .update_card(
                &b.id,
                "KB-1",
                &CardPatch {
                    due: Some("2026-06-20".into()),
                    ..Default::default()
                },
                "t",
            )
            .unwrap();
        let c = store.card_by_key(&b.id, "KB-1").unwrap().unwrap();
        assert_eq!(format_date(c.due_date.unwrap()), "2026-06-20");
        // Empty string clears it.
        store
            .update_card(
                &b.id,
                "KB-1",
                &CardPatch {
                    due: Some("".into()),
                    ..Default::default()
                },
                "t",
            )
            .unwrap();
        assert!(store
            .card_by_key(&b.id, "KB-1")
            .unwrap()
            .unwrap()
            .due_date
            .is_none());
    }
}
