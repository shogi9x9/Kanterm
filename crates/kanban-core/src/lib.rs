//! kanban-core: the single source of truth for the kanban board.
//!
//! Both the TUI (`kanban-tui`) and the MCP server (`kanban-mcp`) depend on this
//! crate and nothing else touches the database directly. All schema rules,
//! migrations, and write logic live here so the two frontends can never drift.

mod activity;
mod agents;
mod board_template;
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

pub use board_template::BoardColumnTemplate;
pub use database::Store;
pub(crate) use dates::MS_PER_DAY;
pub use dates::{format_date, now_ms, parse_date, today_start_ms};
pub use domain::{
    card_is_stale, classify_graph_node, classify_work, priority_badge, priority_label, ActivityLog,
    AgentRegistration, AgentRegistrationResult, Board, Card, CardCreateDraft, CardDependency,
    CardPatch, CardReadiness, Column, DependencyBlockedCard, DependencyBlocker,
    DependencyStagePlan, GraphNodeState, HumanIntervention, Label, Memory, MemoryPatch, WorkState,
    PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_NORMAL, STALE_CARD_MS,
};

/// Bump this whenever a migration is added. Stored in SQLite `PRAGMA user_version`.
pub const SCHEMA_VERSION: i64 = 18;

pub const BACKLOG_BOARD_COLUMNS: &[&str] = &["Backlog"];
pub const PROTECTED_BOARD_SLUG: &str = "backlog";

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
mod tests;
