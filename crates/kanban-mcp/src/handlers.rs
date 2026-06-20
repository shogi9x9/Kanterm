mod agents;
mod boards;
mod cards;
mod columns;
mod memories;

pub(crate) use agents::register_agent;
pub(crate) use boards::manage_boards;
pub(crate) use cards::{
    create_card, dependency_graph, get_board, get_card, list_cards, update_card,
};
pub(crate) use cards::{create_card_in_backlog, create_cards};
pub(crate) use columns::manage_columns;
pub(crate) use memories::{recall_memories, record_memory};

pub(crate) fn status(store: &kanban_core::Store, default_board_id: &str, db_path: &str) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|err| format!("unavailable: {err}"));
    let board = store
        .board_by_id_or_slug(default_board_id)
        .ok()
        .map(|board| format!("{} ({})", board.slug, board.name))
        .unwrap_or_else(|| "-".into());
    format!(
        "kanban_mcp_status:\nversion: {}\nschema_version: {}\ndb_path: {}\nworking_directory: {}\ndefault_board: {}",
        env!("CARGO_PKG_VERSION"),
        kanban_core::SCHEMA_VERSION,
        db_path,
        cwd,
        board
    )
}
