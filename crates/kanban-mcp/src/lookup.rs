use std::collections::HashMap;

use kanban_core::{BoardColumnTemplate, Store, PROTECTED_BOARD_SLUG};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};

pub(crate) struct CreateBoardDestination {
    pub(crate) id: String,
    pub(crate) slug: String,
    pub(crate) created: bool,
}

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

/// Resolve a project-board creation target, creating unknown project boards.
pub(crate) fn resolve_or_create_project_board(
    store: &mut Store,
    board: Option<&str>,
    caller: &str,
) -> Result<CreateBoardDestination, ErrorData> {
    let board = board.ok_or_else(|| {
        bad_param(
            "`board` is required; pass an existing project board slug or a new project board name. \
             To create in Backlog, use create_card_in_backlog.",
        )
    })?;
    if board == PROTECTED_BOARD_SLUG {
        return Err(bad_param(format!(
            "{caller} cannot target the Backlog board; use create_card_in_backlog instead."
        )));
    }
    if let Some(existing) = store.board_by_slug(board).map_err(internal)? {
        return Ok(CreateBoardDestination {
            id: existing.id,
            slug: existing.slug,
            created: false,
        });
    }
    let created = store
        .create_board(board, BoardColumnTemplate::DEFAULT_PROJECT)
        .map_err(internal)?;
    Ok(CreateBoardDestination {
        id: created.id,
        slug: created.slug,
        created: true,
    })
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
