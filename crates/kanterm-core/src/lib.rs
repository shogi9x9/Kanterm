//! kanterm-core: the single source of truth for the kanban board.
//!
//! Both the TUI (`kanterm`) and the MCP server (`kanterm-mcp`) depend on this
//! crate and nothing else touches the database directly. All schema rules,
//! migrations, and write logic live here so the two frontends can never drift.

mod activity;
mod agent_task_attempts;
mod agent_work_packet;
mod agents;
mod board_execution_prompt;
mod board_template;
mod boards;
mod cards;
mod columns;
mod config;
mod database;
mod dates;
mod dependencies;
mod domain;
mod execution_prompt;
mod export;
mod handoffs;
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

pub use agent_task_attempts::AgentTaskAttempt;
pub use agent_work_packet::{
    AgentWorkPacket, AgentWorkPacketAttemptDelta, AgentWorkPacketProfile,
    AgentWorkPacketResumeDelta, AGENT_WORK_PACKET_VERSION, MAX_RESUME_DELTA_CHARS,
};
pub use board_execution_prompt::{
    build_board_execution_prompt, BoardCardProgress, BoardExecutionPromptSnapshot,
    BOARD_EXECUTION_PROMPT_VERSION,
};
pub use board_template::BoardColumnTemplate;
pub use config::*;
pub use database::Store;
pub(crate) use dates::MS_PER_DAY;
pub use dates::{format_date, now_ms, parse_date, today_start_ms};
pub use domain::{
    card_is_stale, classify_graph_node, classify_work, priority_badge, priority_label, ActivityLog,
    AgentHandoff, AgentRegistration, AgentRegistrationResult, Board, Card, CardCreateDraft,
    CardDependency, CardPatch, CardReadiness, Column, DependencyBlockedCard, DependencyBlocker,
    DependencyStagePlan, GraphNodeState, HandoffDraft, HandoffListQuery, HandoffStatusPatch,
    HumanIntervention, Label, Memory, MemoryPatch, WorkState, PRIORITY_HIGH, PRIORITY_LOW,
    PRIORITY_NORMAL, STALE_CARD_MS,
};
pub use execution_prompt::{
    build_execution_prompt, build_resume_prompt, build_verification_prompt,
    ExecutionPromptSnapshot, EXECUTION_PROMPT_VERSION, MAX_EXECUTION_PROMPT_BYTES,
};

/// Bump this whenever a migration is added. Stored in SQLite `PRAGMA user_version`.
pub const SCHEMA_VERSION: i64 = 21;

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
