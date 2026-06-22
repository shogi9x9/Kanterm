use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use kanban_core::{Board, BoardColumnTemplate};

use crate::app::App;
use crate::mode::Mode;

impl App {
    /// Archived boards, most recently created first kept in store order.
    pub(crate) fn archived_boards(&self) -> Result<Vec<Board>> {
        Ok(self
            .store
            .list_boards_all()?
            .into_iter()
            .filter(|b| b.archived_at.is_some())
            .collect())
    }

    pub(crate) fn on_board_archive_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::BoardArchive {
            board_id,
            board_name,
        } = &self.mode
        else {
            return Ok(());
        };
        let board_id = board_id.clone();
        let board_name = board_name.clone();
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.store.archive_board(&board_id)?;
                self.boards = self.store.list_boards()?;
                // The Backlog board always exists and can't be archived, so a
                // remaining active board is guaranteed.
                if let Some(next) = self.boards.iter().find(|b| b.id != board_id).cloned() {
                    self.switch_board(next)?;
                }
                self.mode = Mode::Normal;
                self.status = format!("archived board '{board_name}' (U to restore)");
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.status = "cancelled".into();
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_board_switcher_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::BoardSwitcher { ref cursor } = &self.mode else {
            return Ok(());
        };
        self.boards = self.store.list_boards()?;
        if self.boards.is_empty() {
            self.mode = Mode::Normal;
            self.status = "no boards".into();
            return Ok(());
        }
        let cursor = (*cursor).min(self.boards.len() - 1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => self.mode = Mode::Normal,
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::BoardSwitcher {
                    cursor: (cursor + 1).min(self.boards.len() - 1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::BoardSwitcher {
                    cursor: cursor.saturating_sub(1),
                };
            }
            KeyCode::Char('J') => self.reorder_board_from_switcher(cursor, 1)?,
            KeyCode::Char('K') => self.reorder_board_from_switcher(cursor, -1)?,
            KeyCode::Enter => {
                let board = self.boards[cursor].clone();
                self.switch_board(board.clone())?;
                self.mode = Mode::Normal;
                self.status = format!("board: {}", board.name);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_board_template_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::BoardTemplatePicker { name, cursor } = &self.mode else {
            return Ok(());
        };
        let name = name.clone();
        let cursor = *cursor;
        let templates = BoardColumnTemplate::ALL;
        let cursor = cursor.min(templates.len().saturating_sub(1));
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
                self.status = "cancelled".into();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::BoardTemplatePicker {
                    name,
                    cursor: (cursor + 1).min(templates.len() - 1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::BoardTemplatePicker {
                    name,
                    cursor: cursor.saturating_sub(1),
                };
            }
            KeyCode::Enter => {
                let template = templates[cursor];
                let board = self.store.create_board(&name, template)?;
                self.boards = self.store.list_boards()?;
                self.switch_board(board)?;
                self.status = format!("created board {} ({})", self.board.name, template.key());
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn reorder_board_from_switcher(&mut self, cursor: usize, dir: i32) -> Result<()> {
        let Some(board) = self.boards.get(cursor).cloned() else {
            return Ok(());
        };
        self.store.reorder_board(&board.id, dir)?;
        self.boards = self.store.list_boards()?;
        if let Some(pos) = self.boards.iter().position(|b| b.id == board.id) {
            self.mode = Mode::BoardSwitcher { cursor: pos };
        }
        self.status = format!("reordered board '{}'", board.name);
        Ok(())
    }

    pub(crate) fn on_board_unarchive_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::BoardUnarchive { ref cursor } = &self.mode else {
            return Ok(());
        };
        let cursor = *cursor;
        let archived = self.archived_boards()?;
        if archived.is_empty() {
            self.mode = Mode::Normal;
            self.status = "no archived boards".into();
            return Ok(());
        }
        let cursor = cursor.min(archived.len() - 1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Normal,
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::BoardUnarchive {
                    cursor: (cursor + 1).min(archived.len() - 1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::BoardUnarchive {
                    cursor: cursor.saturating_sub(1),
                };
            }
            KeyCode::Enter => {
                let board = archived[cursor].clone();
                self.store.unarchive_board(&board.id)?;
                self.boards = self.store.list_boards()?;
                self.switch_board(board.clone())?;
                self.mode = Mode::Normal;
                self.status = format!("unarchived board '{}'", board.name);
            }
            KeyCode::Char('d') => {
                let board = &archived[cursor];
                self.mode = Mode::BoardDelete {
                    board_id: board.id.clone(),
                    board_name: board.name.clone(),
                    confirm: String::new(),
                };
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_board_delete_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::BoardDelete {
            board_id,
            board_name,
            confirm,
        } = &mut self.mode
        else {
            return Ok(());
        };
        let board_id = board_id.clone();
        let board_name = board_name.clone();
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::BoardUnarchive { cursor: 0 };
                self.status = "cancelled".into();
            }
            KeyCode::Backspace => {
                confirm.pop();
            }
            KeyCode::Char(c) => {
                confirm.push(c);
            }
            KeyCode::Enter => {
                if confirm != "delete" {
                    self.status = "type `delete` to confirm".into();
                    return Ok(());
                }
                // The store enforces archived-only deletion; surface any race
                // (e.g. unarchived/deleted by another process) as a status line.
                match self.store.delete_board(&board_id) {
                    Ok(()) => self.status = format!("deleted board '{board_name}'"),
                    Err(e) => self.status = format!("delete failed: {e}"),
                }
                self.boards = self.store.list_boards()?;
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
}
