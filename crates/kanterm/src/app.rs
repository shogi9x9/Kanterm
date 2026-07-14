use crate::mode::{ExecutionDashboardState, ExecutionDashboardView, Mode};
use anyhow::Result;
use kanterm_core::{now_ms, Board, Card, Label, Store};
use std::collections::HashMap;

mod actions;
mod execution_dashboard;
pub(crate) mod render;
mod render_agent_metadata;
mod render_board;
mod render_board_dialogs;
mod render_card_move_dialogs;
mod render_dependency_graph;
mod render_detail;
mod render_dialogs;
mod render_execution_dashboard;
mod render_execution_timeline;
mod render_memory_dialogs;
mod render_popups;
mod render_status;

const ACTOR: &str = "tui";
const UI_FOCUS: &str = "tui.focus";
const UI_SELECTED: &str = "tui.selected_key";
const UI_BOARD: &str = "tui.board";

fn claim_is_active(card: &Card) -> bool {
    matches!(card.lease_expires_at, Some(expires_at) if expires_at > now_ms())
}

/// Labels not attached to any card within this window are hidden from the
/// picker's suggestion list (still removable if already on the card).
const LABEL_RECENCY_MS: i64 = 30 * 24 * 60 * 60 * 1000;

pub(crate) struct App {
    store: Store,
    board: Board,
    boards: Vec<Board>,
    columns: Vec<kanterm_core::Column>,
    cards: Vec<Card>,
    labels: HashMap<String, Vec<Label>>,
    focus: usize,
    cursors: Vec<usize>,
    col_cursor: usize,
    filter: Option<String>,
    mode: Mode,
    /// Dashboard tab to restore after closing a card opened from an execution
    /// view. Kanban card details continue to return to Kanban.
    detail_return_dashboard: Option<ExecutionDashboardState>,
    status: String,
    /// Last-seen SQLite data_version; lets us notice agent/MCP writes and
    /// auto-refresh without polling table contents.
    data_version: i64,
    should_quit: bool,
}

impl App {
    pub(crate) fn new(store: Store, board: Board) -> Result<App> {
        let columns = store.columns(&board.id)?;
        let cursors = vec![0; columns.len().max(1)];
        let mut app = App {
            store,
            board,
            boards: Vec::new(),
            columns,
            cards: Vec::new(),
            labels: HashMap::new(),
            focus: 0,
            cursors,
            col_cursor: 0,
            filter: None,
            mode: Mode::ExecutionDashboard(ExecutionDashboardState::new(
                ExecutionDashboardView::List,
                0,
                0,
            )),
            detail_return_dashboard: None,
            status: String::new(),
            data_version: 0,
            should_quit: false,
        };
        app.boards = app.store.list_boards()?;
        // Restore the last-used board if it still exists.
        let restore = app
            .store
            .get_ui_state(UI_BOARD)
            .ok()
            .flatten()
            .and_then(|slug| app.boards.iter().find(|b| b.slug == slug).cloned());
        match restore {
            Some(b) if b.id != app.board.id => app.switch_board(b)?,
            _ => app.reload()?,
        }
        app.restore_ui_state();
        app.data_version = app.store.data_version().unwrap_or(0);
        Ok(app)
    }
}
