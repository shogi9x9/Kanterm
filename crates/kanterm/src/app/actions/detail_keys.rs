use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::editor::Editor;
use crate::mode::{ArchiveBack, InputKind, Mode};
use kanterm_core::format_date;

impl App {
    pub(crate) fn on_detail_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::Detail {
            key: card_key,
            ref scroll,
        } = &self.mode
        else {
            return Ok(());
        };
        let card_key = card_key.clone();
        let scroll = *scroll;
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: scroll.saturating_add(1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::PageDown => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: scroll.saturating_add(10),
                };
            }
            KeyCode::PageUp => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: scroll.saturating_sub(10),
                };
            }
            KeyCode::Home => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: 0,
                };
            }
            KeyCode::End => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: u16::MAX,
                };
            }
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Normal,
            KeyCode::Char('e') => {
                if let Some(c) = self.card_by_key(&card_key) {
                    let title = c.title.clone();
                    self.mode = Mode::Input {
                        kind: InputKind::EditTitle {
                            key: card_key,
                            expected_updated_at: c.updated_at,
                        },
                        buffer: title,
                    };
                }
            }
            KeyCode::Char('b') => {
                if let Some(c) = self.card_by_key(&card_key) {
                    let editor = Editor::new(&c.body);
                    self.mode = Mode::BodyEdit {
                        key: card_key,
                        editor,
                        expected_updated_at: c.updated_at,
                    };
                }
            }
            KeyCode::Char('a') => {
                if let Some(c) = self.card_by_key(&card_key) {
                    let cur = c.assignee.clone().unwrap_or_default();
                    self.mode = Mode::Input {
                        kind: InputKind::EditAssignee {
                            key: card_key,
                            expected_updated_at: c.updated_at,
                        },
                        buffer: cur,
                    };
                }
            }
            KeyCode::Char('D') => {
                if let Some(c) = self.card_by_key(&card_key) {
                    let cur = c.due_date.map(format_date).unwrap_or_default();
                    self.mode = Mode::Input {
                        kind: InputKind::EditDue {
                            key: card_key,
                            expected_updated_at: c.updated_at,
                        },
                        buffer: cur,
                    };
                }
            }
            KeyCode::Char('t') => self.open_label_picker(card_key)?,
            KeyCode::Char('p') => {
                self.cycle_priority_key(&card_key)?;
            }
            KeyCode::Char('d') => {
                self.prompt_archive_key(card_key, ArchiveBack::Detail);
            }
            KeyCode::Char('x') => {
                if let Some(c) = self.card_by_key(&card_key) {
                    self.mode = Mode::Input {
                        kind: InputKind::CompleteWithNote {
                            key: card_key,
                            expected_updated_at: c.updated_at,
                        },
                        buffer: String::new(),
                    };
                }
            }
            KeyCode::Char('m') => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: 0,
                };
            }
            KeyCode::Char('M') => {
                self.open_card_board_move(card_key, ArchiveBack::Detail);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_agent_metadata_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::AgentMetadata {
            key: card_key,
            ref scroll,
        } = &self.mode
        else {
            return Ok(());
        };
        let card_key = card_key.clone();
        let scroll = *scroll;
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: scroll.saturating_add(1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::PageDown => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: scroll.saturating_add(10),
                };
            }
            KeyCode::PageUp => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: scroll.saturating_sub(10),
                };
            }
            KeyCode::Home => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: 0,
                };
            }
            KeyCode::End => {
                self.mode = Mode::AgentMetadata {
                    key: card_key,
                    scroll: u16::MAX,
                };
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => {
                self.mode = Mode::Detail {
                    key: card_key,
                    scroll: 0,
                };
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_dependency_graph_key(&mut self, key: KeyEvent) {
        let Mode::DependencyGraph { ref scroll } = &self.mode else {
            return;
        };
        let scroll = *scroll;
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::DependencyGraph {
                    scroll: scroll.saturating_add(1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::DependencyGraph {
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::PageDown => {
                self.mode = Mode::DependencyGraph {
                    scroll: scroll.saturating_add(10),
                };
            }
            KeyCode::PageUp => {
                self.mode = Mode::DependencyGraph {
                    scroll: scroll.saturating_sub(10),
                };
            }
            KeyCode::Home => {
                self.mode = Mode::DependencyGraph { scroll: 0 };
            }
            KeyCode::End => {
                self.mode = Mode::DependencyGraph { scroll: u16::MAX };
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('g') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }
}
