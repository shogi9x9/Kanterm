use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{CardActionBack, ExecutionDashboardView, InputKind, Mode, ViewBack};
use kanterm_core::PROTECTED_BOARD_SLUG;

impl App {
    pub(crate) fn on_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('H') => self.move_card(-1)?,
            KeyCode::Char('L') => self.move_card(1)?,
            KeyCode::Char('J') => self.reorder(1)?,
            KeyCode::Char('K') => self.reorder(-1)?,
            KeyCode::Char('h') | KeyCode::Left => self.focus_delta(-1),
            KeyCode::Char('l') | KeyCode::Right => self.focus_delta(1),
            KeyCode::Char('j') | KeyCode::Down => self.cursor_delta(1),
            KeyCode::Char('k') | KeyCode::Up => self.cursor_delta(-1),
            KeyCode::Enter => {
                if let Some(c) = self.selected_card() {
                    let key = c.key.clone();
                    self.detail_return_dashboard = None;
                    self.mode = Mode::Detail { key, scroll: 0 };
                }
            }
            KeyCode::Char('n') => {
                self.mode = Mode::Input {
                    kind: InputKind::NewCard,
                    buffer: String::new(),
                };
            }
            KeyCode::Char('N') => {
                self.mode = Mode::Input {
                    kind: InputKind::NewBoard,
                    buffer: String::new(),
                };
            }
            KeyCode::Tab => self.open_execution_dashboard(ExecutionDashboardView::List),
            KeyCode::BackTab => self.open_execution_dashboard(ExecutionDashboardView::Timeline),
            KeyCode::Char('b') => {
                let cursor = self
                    .boards
                    .iter()
                    .position(|b| b.id == self.board.id)
                    .unwrap_or(0);
                self.mode = Mode::BoardSwitcher {
                    cursor,
                    back: ViewBack::Normal,
                };
            }
            KeyCode::Char('C') => self.open_board_execution_prompt(ViewBack::Normal),
            KeyCode::Char('i') => {
                self.mode = Mode::Input {
                    kind: InputKind::EditBoardContext,
                    buffer: self.board.agent_context.clone().unwrap_or_default(),
                };
            }
            KeyCode::Char('c') => {
                if self.board.slug == PROTECTED_BOARD_SLUG {
                    self.status = "Backlog board has exactly one Backlog column".into();
                } else {
                    self.col_cursor = self.focus.min(self.columns.len().saturating_sub(1));
                    self.mode = Mode::ColumnManager;
                }
            }
            KeyCode::Char('e') => self.start_edit_title(),
            KeyCode::Char('/') => {
                let buffer = self.filter.clone().unwrap_or_default();
                self.mode = Mode::Input {
                    kind: InputKind::Filter,
                    buffer,
                };
            }
            KeyCode::Char('v') => self.jump_to_human_intervention(),
            KeyCode::Char('w') => self.jump_to_next_work(),
            KeyCode::Char('g') => {
                self.mode = Mode::DependencyGraph { scroll: 0 };
            }
            KeyCode::Char('p') => self.cycle_priority()?,
            KeyCode::Char('u') => self.undo_last_card_update()?,
            KeyCode::Char('M') => {
                if let Some(card) = self.selected_card() {
                    self.open_card_board_move(
                        card.key.clone(),
                        CardActionBack::View(ViewBack::Normal),
                    );
                }
            }
            KeyCode::Char('d') => self.prompt_archive_selected(),
            KeyCode::Char('D') => {
                if self.board.slug == PROTECTED_BOARD_SLUG {
                    self.status = "cannot archive the Backlog board".into();
                } else {
                    self.mode = Mode::BoardArchive {
                        board_id: self.board.id.clone(),
                        board_name: self.board.name.clone(),
                        back: ViewBack::Normal,
                    };
                }
            }
            KeyCode::Char('m') => {
                self.mode = Mode::MemoryBrowser { cursor: 0 };
            }
            KeyCode::Char('U') => {
                if self.archived_boards()?.is_empty() {
                    self.status = "no archived boards".into();
                } else {
                    self.mode = Mode::BoardUnarchive { cursor: 0 };
                }
            }
            KeyCode::Char('r') => {
                self.reload()?;
                self.status = "reloaded".into();
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn start_edit_title(&mut self) {
        if let Some(c) = self.selected_card() {
            let (key, title) = (c.key.clone(), c.title.clone());
            self.mode = Mode::Input {
                kind: InputKind::EditTitle {
                    key,
                    expected_updated_at: c.updated_at,
                },
                buffer: title,
            };
        }
    }
}
