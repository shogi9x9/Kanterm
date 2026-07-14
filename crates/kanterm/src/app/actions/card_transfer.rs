use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use kanterm_core::{Board, CardPatch};

use crate::app::App;
use crate::mode::{CardActionBack, Mode};

impl App {
    pub(crate) fn card_move_destinations(&self) -> Vec<Board> {
        self.boards
            .iter()
            .filter(|b| b.id != self.board.id)
            .cloned()
            .collect()
    }

    pub(crate) fn open_card_board_move(&mut self, key: String, back: CardActionBack) {
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
                let from_backlog = self.board.slug == kanterm_core::PROTECTED_BOARD_SLUG;
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

    fn return_from_card_board_move(&mut self, key: String, back: CardActionBack) {
        self.mode = back.return_mode(Some(key));
    }
}
