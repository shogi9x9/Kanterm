use anyhow::Result;

use crate::{Board, BoardColumnTemplate, Store, BACKLOG_BOARD_COLUMNS, PROTECTED_BOARD_SLUG};

mod create;
mod lifecycle;
mod order;
mod read;
#[cfg(test)]
mod tests;

impl Store {
    /// Return the default board, creating it if absent.
    /// This is the board the TUI/MCP fall back to when none is specified.
    pub fn ensure_default_board(&mut self) -> Result<Board> {
        create::ensure_system_board(
            self,
            "Backlog",
            PROTECTED_BOARD_SLUG,
            "KB",
            BACKLOG_BOARD_COLUMNS,
        )
    }

    /// Create a new board from a display name, deriving a unique slug and a key
    /// prefix automatically. Columns are selected by the requested template.
    pub fn create_board(&mut self, name: &str, template: BoardColumnTemplate) -> Result<Board> {
        create::create_board(self, name, template)
    }

    /// Active (non-archived) boards. This is what board pickers should show.
    pub fn list_boards(&self) -> Result<Vec<Board>> {
        read::list_boards(&self.conn)
    }

    /// Every board, archived ones included.
    pub fn list_boards_all(&self) -> Result<Vec<Board>> {
        read::list_boards_all(&self.conn)
    }

    /// Archive a board: it disappears from `list_boards` but keeps all its
    /// columns/cards. The Backlog board cannot be archived.
    pub fn archive_board(&mut self, board_id: &str) -> Result<()> {
        lifecycle::archive_board(self, board_id)
    }

    pub fn unarchive_board(&mut self, board_id: &str) -> Result<()> {
        lifecycle::unarchive_board(self, board_id)
    }

    /// Delete a board and everything on it (columns/cards cascade).
    /// Only archived boards can be deleted.
    pub fn delete_board(&mut self, board_id: &str) -> Result<()> {
        lifecycle::delete_board(self, board_id)
    }

    /// Move an active board earlier (-1) or later (+1) by swapping sort_order
    /// with its active neighbour. No-op at the ends.
    pub fn reorder_board(&mut self, board_id: &str, dir: i32) -> Result<()> {
        order::reorder_board(self, board_id, dir)
    }

    /// Set or clear board-level agent execution guidance. Empty/whitespace text clears it.
    pub fn update_board_agent_context(
        &mut self,
        board_id: &str,
        agent_context: Option<&str>,
    ) -> Result<Board> {
        lifecycle::update_board_agent_context(self, board_id, agent_context)
    }

    pub fn board_by_slug(&self, slug: &str) -> Result<Option<Board>> {
        read::board_by_slug(&self.conn, slug)
    }

    pub fn board_by_id_or_slug(&self, value: &str) -> Result<Board> {
        self.board_by_slug(value)?
            .map(Ok)
            .unwrap_or_else(|| read::board_by_id(&self.conn, value))
    }
}
