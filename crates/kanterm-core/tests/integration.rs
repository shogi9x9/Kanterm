//! Integration tests against the public `kanterm-core` API, including the
//! concurrency story (WAL + BEGIN IMMEDIATE + busy_timeout) that the TUI and the
//! MCP server rely on when they write to the same database at once.

mod common;

#[path = "integration/agent_workflow/mod.rs"]
mod agent_workflow;
#[path = "integration/boards_cards/mod.rs"]
mod boards_cards;
#[path = "integration/dependencies.rs"]
mod dependencies;
#[path = "integration/memories.rs"]
mod memories;
#[path = "integration/migrations.rs"]
mod migrations;
