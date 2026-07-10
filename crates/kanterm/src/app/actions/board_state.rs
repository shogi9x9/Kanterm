use anyhow::Result;
use kanterm_core::Board;

use crate::app::App;

impl App {
    pub(crate) fn switch_board(&mut self, board: Board) -> Result<()> {
        self.board = board;
        self.columns = self.store.columns(&self.board.id)?;
        self.cursors = vec![0; self.columns.len().max(1)];
        self.focus = 0;
        self.filter = None;
        self.reload()
    }

    /// Reload columns after a structural change, keeping cursors in range.
    pub(crate) fn refresh_columns(&mut self) -> Result<()> {
        self.columns = self.store.columns(&self.board.id)?;
        if self.cursors.len() != self.columns.len() {
            self.cursors = vec![0; self.columns.len().max(1)];
        }
        if self.focus >= self.columns.len() {
            self.focus = self.columns.len().saturating_sub(1);
        }
        if self.col_cursor >= self.columns.len() {
            self.col_cursor = self.columns.len().saturating_sub(1);
        }
        self.reload()
    }
}
