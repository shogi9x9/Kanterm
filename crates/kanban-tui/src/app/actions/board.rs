use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use kanban_core::{Board, BoardColumnTemplate, CardPatch};

use crate::app::App;
use crate::mode::{ArchiveBack, InputKind, Mode};

impl App {
    pub(crate) fn switch_board(&mut self, board: Board) -> Result<()> {
        self.board = board;
        self.columns = self.store.columns(&self.board.id)?;
        self.cursors = vec![0; self.columns.len().max(1)];
        self.focus = 0;
        self.filter = None;
        self.reload()
    }

    pub(crate) fn cycle_board(&mut self) -> Result<()> {
        if self.boards.len() <= 1 {
            return Ok(());
        }
        let i = self
            .boards
            .iter()
            .position(|b| b.id == self.board.id)
            .unwrap_or(0);
        let next = self.boards[(i + 1) % self.boards.len()].clone();
        self.switch_board(next)?;
        self.status = format!("board: {}", self.board.name);
        Ok(())
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

    pub(crate) fn on_columns_key(&mut self, key: KeyEvent) -> Result<()> {
        let last = self.columns.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('c') => self.mode = Mode::Normal,
            KeyCode::Down | KeyCode::Char('j') => self.col_cursor = (self.col_cursor + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => self.col_cursor = self.col_cursor.saturating_sub(1),
            KeyCode::Char('J') => self.reorder_column(1)?,
            KeyCode::Char('K') => self.reorder_column(-1)?,
            KeyCode::Char('a') | KeyCode::Char('n') => {
                self.mode = Mode::Input {
                    kind: InputKind::NewColumn,
                    buffer: String::new(),
                };
            }
            KeyCode::Char('r') => {
                if let Some(col) = self.columns.get(self.col_cursor) {
                    let (id, name) = (col.id.clone(), col.name.clone());
                    self.mode = Mode::Input {
                        kind: InputKind::RenameColumn(id),
                        buffer: name,
                    };
                }
            }
            KeyCode::Char('d') => {
                if self.columns.len() <= 1 {
                    self.status = "cannot delete the only column".into();
                } else if let Some(col) = self.columns.get(self.col_cursor) {
                    self.mode = Mode::ColumnDelete {
                        victim_id: col.id.clone(),
                        cursor: 0,
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn reorder_column(&mut self, dir: i32) -> Result<()> {
        let Some(col) = self.columns.get(self.col_cursor) else {
            return Ok(());
        };
        let id = col.id.clone();
        self.store.reorder_column(&self.board.id, &id, dir)?;
        self.refresh_columns()?;
        // Follow the moved column with the cursor.
        if let Some(pos) = self.columns.iter().position(|c| c.id == id) {
            self.col_cursor = pos;
        }
        Ok(())
    }

    /// Columns other than the one being deleted, as (id, name) pairs.
    pub(crate) fn delete_destinations(&self, victim_id: &str) -> Vec<(String, String)> {
        self.columns
            .iter()
            .filter(|c| c.id != victim_id)
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect()
    }

    pub(crate) fn on_column_delete_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::ColumnDelete {
            victim_id,
            ref cursor,
        } = &self.mode
        else {
            return Ok(());
        };
        let victim_id = victim_id.clone();
        let cursor = *cursor;
        let dests = self.delete_destinations(&victim_id);
        match key.code {
            KeyCode::Esc => self.mode = Mode::ColumnManager,
            KeyCode::Down | KeyCode::Char('j') => {
                let c = (cursor + 1).min(dests.len().saturating_sub(1));
                self.mode = Mode::ColumnDelete {
                    victim_id,
                    cursor: c,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::ColumnDelete {
                    victim_id,
                    cursor: cursor.saturating_sub(1),
                };
            }
            KeyCode::Enter => {
                if let Some((dest_id, dest_name)) = dests.get(cursor) {
                    self.store
                        .delete_column(&self.board.id, &victim_id, dest_id)?;
                    self.refresh_columns()?;
                    self.status = format!("column deleted; cards moved to {dest_name}");
                }
                self.mode = Mode::ColumnManager;
            }
            _ => {}
        }
        Ok(())
    }

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

    pub(crate) fn card_move_destinations(&self) -> Vec<Board> {
        self.boards
            .iter()
            .filter(|b| b.id != self.board.id)
            .cloned()
            .collect()
    }

    pub(crate) fn open_card_board_move(&mut self, key: String, back: ArchiveBack) {
        if self.card_move_destinations().is_empty() {
            self.status = "no other active boards".into();
            return;
        }
        self.mode = Mode::CardBoardMove {
            key,
            cursor: 0,
            back,
        };
    }

    pub(crate) fn on_card_board_move_key(&mut self, key_event: KeyEvent) -> Result<()> {
        let Mode::CardBoardMove { key, cursor, back } = &self.mode else {
            return Ok(());
        };
        let card_key = key.clone();
        let cursor = *cursor;
        let back = *back;
        self.boards = self.store.list_boards()?;
        let destinations = self.card_move_destinations();
        if destinations.is_empty() {
            self.return_from_card_board_move(card_key, back);
            self.status = "no other active boards".into();
            return Ok(());
        }
        let cursor = cursor.min(destinations.len() - 1);
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('M') => {
                self.return_from_card_board_move(card_key, back);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::CardBoardMove {
                    key: card_key,
                    cursor: (cursor + 1).min(destinations.len() - 1),
                    back,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::CardBoardMove {
                    key: card_key,
                    cursor: cursor.saturating_sub(1),
                    back,
                };
            }
            KeyCode::Enter => {
                let destination = destinations[cursor].clone();
                let columns = self.store.columns(&destination.id)?;
                if columns.is_empty() {
                    self.status = format!("board '{}' has no columns", destination.name);
                    return Ok(());
                }
                self.mode = Mode::CardColumnMove {
                    key: card_key,
                    board_id: destination.id,
                    board_name: destination.name,
                    cursor: 0,
                    back,
                };
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_card_column_move_key(&mut self, key_event: KeyEvent) -> Result<()> {
        let Mode::CardColumnMove {
            key,
            board_id,
            board_name,
            cursor,
            back,
        } = &self.mode
        else {
            return Ok(());
        };
        let card_key = key.clone();
        let board_id = board_id.clone();
        let board_name = board_name.clone();
        let cursor = *cursor;
        let back = *back;
        let columns = self.store.columns(&board_id)?;
        if columns.is_empty() {
            self.return_from_card_board_move(card_key, back);
            self.status = format!("board '{board_name}' has no columns");
            return Ok(());
        }
        let cursor = cursor.min(columns.len() - 1);
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('M') => {
                self.return_from_card_board_move(card_key, back);
            }
            KeyCode::Char('b') => {
                self.mode = Mode::CardBoardMove {
                    key: card_key,
                    cursor: 0,
                    back,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::CardColumnMove {
                    key: card_key,
                    board_id,
                    board_name,
                    cursor: (cursor + 1).min(columns.len() - 1),
                    back,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::CardColumnMove {
                    key: card_key,
                    board_id,
                    board_name,
                    cursor: cursor.saturating_sub(1),
                    back,
                };
            }
            KeyCode::Enter => {
                let from_backlog = self.board.slug == kanban_core::PROTECTED_BOARD_SLUG;
                self.boards = self.store.list_boards()?;
                let destination = self.boards.iter().find(|b| b.id == board_id).cloned();
                let Some(destination) = destination else {
                    self.status = format!("board '{board_name}' not found");
                    return Ok(());
                };
                let column_name = columns[cursor].name.clone();
                let moved = self.store.update_card(
                    &self.board.id,
                    &card_key,
                    &CardPatch {
                        move_to_board: Some(destination.id.clone()),
                        column: Some(column_name.clone()),
                        ..Default::default()
                    },
                    super::super::ACTOR,
                )?;
                self.switch_board(destination.clone())?;
                self.select_key(&moved.key);
                self.mode = Mode::Normal;
                self.status = if from_backlog {
                    format!("sent {card_key} -> {} as {}", destination.name, moved.key)
                } else {
                    format!("moved {card_key} -> {} as {}", destination.name, moved.key)
                };
            }
            _ => {}
        }
        Ok(())
    }

    fn return_from_card_board_move(&mut self, key: String, back: ArchiveBack) {
        self.mode = match back {
            ArchiveBack::Normal => Mode::Normal,
            ArchiveBack::Detail => Mode::Detail { key, scroll: 0 },
        };
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
