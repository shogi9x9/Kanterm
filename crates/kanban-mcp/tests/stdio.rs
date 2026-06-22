//! End-to-end test of the MCP server: spawn the real binary, speak
//! line-delimited JSON-RPC over stdio, and assert on tool behaviour.
//!
//! Cargo provides the built binary path via `CARGO_BIN_EXE_kanban-mcp`.

#[path = "stdio/board_admin.rs"]
mod board_admin;
#[path = "stdio/card_updates.rs"]
mod card_updates;
#[path = "stdio/create.rs"]
mod create;
#[path = "stdio/dependencies.rs"]
mod dependencies;
#[path = "stdio/memories.rs"]
mod memories;
#[path = "stdio/support.rs"]
mod support;
#[path = "stdio/tools.rs"]
mod tools;
