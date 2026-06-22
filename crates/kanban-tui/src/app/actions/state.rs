use anyhow::Result;

use crate::app::{App, UI_FOCUS, UI_SELECTED};
use crate::mode::Mode;

impl App {
    pub(crate) fn reload(&mut self) -> Result<()> {
        self.cards = self.store.cards(&self.board.id)?;
        self.labels = self.store.labels_by_card(&self.board.id)?;
        if self.cursors.len() != self.columns.len() {
            self.cursors = vec![0; self.columns.len()];
        }
        for i in 0..self.columns.len() {
            let max = self.column_cards(i).len().saturating_sub(1);
            if self.cursors[i] > max {
                self.cursors[i] = max;
            }
        }
        Ok(())
    }

    pub(crate) fn resync_external(&mut self) -> Result<()> {
        self.boards = self.store.list_boards()?;
        if let Some(board) = self.boards.iter().find(|b| b.id == self.board.id).cloned() {
            self.board = board;
            // External writers can change board/column structure, while reload()
            // intentionally only refreshes the card window for hot paths.
            self.refresh_columns()?;
        } else if let Some(board) = self.boards.first().cloned() {
            let previous = self.board.name.clone();
            self.switch_board(board)?;
            self.mode = Mode::Normal;
            self.status = format!(
                "current board '{previous}' is unavailable; switched to {}",
                self.board.name
            );
        }
        Ok(())
    }

    pub(crate) fn restore_ui_state(&mut self) {
        if let Ok(Some(v)) = self.store.get_ui_state(UI_FOCUS) {
            if let Ok(i) = v.parse::<usize>() {
                if i < self.columns.len() {
                    self.focus = i;
                }
            }
        }
        if let Ok(Some(key)) = self.store.get_ui_state(UI_SELECTED) {
            self.select_key(&key);
        }
    }

    pub(crate) fn save_ui_state(&self) {
        let _ = self
            .store
            .set_ui_state(crate::app::UI_BOARD, &self.board.slug);
        let _ = self.store.set_ui_state(UI_FOCUS, &self.focus.to_string());
        if let Some(c) = self.selected_card() {
            let _ = self.store.set_ui_state(UI_SELECTED, &c.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_core::{BoardColumnTemplate, Store};

    #[test]
    fn resync_external_refreshes_board_metadata_and_columns() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();
        let mut app = App::new(store, board.clone()).unwrap();

        app.store
            .update_board_agent_context(&board.id, Some("Run checks before closing."))
            .unwrap();
        app.store.add_column(&board.id, "Review").unwrap();

        assert_eq!(app.board.agent_context, None);
        assert!(!app.columns.iter().any(|c| c.name == "Review"));

        app.resync_external().unwrap();

        assert_eq!(
            app.board.agent_context.as_deref(),
            Some("Run checks before closing.")
        );
        assert!(app.columns.iter().any(|c| c.name == "Review"));
    }

    #[test]
    fn resync_external_recovers_when_current_board_disappears() {
        let mut store = Store::open_in_memory().unwrap();
        // The Backlog board always exists in a real store and is the guaranteed
        // fallback target when the viewed board disappears.
        store.ensure_default_board().unwrap();
        let work = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();
        let mut app = App::new(store, work.clone()).unwrap();
        assert_eq!(app.board.id, work.id);

        // An external writer archives the board we're viewing while we sit in a
        // non-Normal mode that points at its now-gone structure.
        app.mode = Mode::ColumnManager;
        app.store.archive_board(&work.id).unwrap();

        app.resync_external().unwrap();

        // We fall back to a surviving board (the always-present Backlog), reset
        // the mode, and surface why the view changed.
        assert_ne!(app.board.id, work.id);
        assert_eq!(app.board.slug, kanban_core::PROTECTED_BOARD_SLUG);
        assert!(matches!(app.mode, Mode::Normal));
        assert!(app.status.contains("unavailable"));
    }
}
