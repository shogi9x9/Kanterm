use std::collections::HashMap;

use kanban_core::Store;
use rmcp::ErrorData;

use crate::error::{bad_param, internal};

pub(crate) fn columns_by_id(
    store: &Store,
    board_id: &str,
) -> Result<HashMap<String, String>, ErrorData> {
    let cols = store.columns(board_id).map_err(internal)?;
    Ok(cols.into_iter().map(|c| (c.id, c.name)).collect())
}

/// Resolve an optional board slug to a board id, defaulting to the Backlog board.
pub(crate) fn resolve_board(
    store: &Store,
    default_board_id: &str,
    slug: Option<&str>,
) -> Result<String, ErrorData> {
    match slug {
        None => Ok(default_board_id.to_string()),
        Some(s) => store
            .board_by_slug(s)
            .map_err(internal)?
            .map(|b| b.id)
            .ok_or_else(|| bad_param(format!("no board '{s}'"))),
    }
}

/// Resolve a column name to its id within a board.
pub(crate) fn resolve_column(
    store: &Store,
    board_id: &str,
    name: &str,
) -> Result<String, ErrorData> {
    store
        .columns(board_id)
        .map_err(internal)?
        .into_iter()
        .find(|c| c.name == name)
        .map(|c| c.id)
        .ok_or_else(|| bad_param(format!("no column '{name}'")))
}
