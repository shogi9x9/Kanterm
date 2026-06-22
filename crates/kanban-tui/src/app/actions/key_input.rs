use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::editor::Editor;
use crate::mode::{ArchiveBack, InputKind, Mode};
use kanban_core::{format_date, PROTECTED_BOARD_SLUG};

impl App {
    pub(crate) fn on_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::Normal => self.on_normal_key(key)?,
            Mode::Detail { .. } => self.on_detail_key(key)?,
            Mode::AgentMetadata { .. } => self.on_agent_metadata_key(key)?,
            Mode::DependencyGraph { .. } => self.on_dependency_graph_key(key),
            Mode::Input { .. } => self.on_input_key(key)?,
            Mode::BodyEdit { .. } => self.on_body_key(key)?,
            Mode::LabelPicker { .. } => self.on_label_key(key)?,
            Mode::ColumnManager => self.on_columns_key(key)?,
            Mode::ColumnDelete { .. } => self.on_column_delete_key(key)?,
            Mode::BoardArchive { .. } => self.on_board_archive_key(key)?,
            Mode::BoardSwitcher { .. } => self.on_board_switcher_key(key)?,
            Mode::BoardTemplatePicker { .. } => self.on_board_template_key(key)?,
            Mode::CardBoardMove { .. } => self.on_card_board_move_key(key)?,
            Mode::CardColumnMove { .. } => self.on_card_column_move_key(key)?,
            Mode::BoardUnarchive { .. } => self.on_board_unarchive_key(key)?,
            Mode::BoardDelete { .. } => self.on_board_delete_key(key)?,
            Mode::MemoryBrowser { .. } => self.on_memory_browser_key(key)?,
            Mode::MemoryDetail { .. } => self.on_memory_detail_key(key)?,
            Mode::MemoryArchive { .. } => self.on_memory_archive_key(key)?,
            Mode::ArchiveConfirm { .. } => self.on_archive_confirm_key(key)?,
        }
        Ok(())
    }

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
                    self.mode = Mode::Detail {
                        key: c.key.clone(),
                        scroll: 0,
                    };
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
            KeyCode::Tab | KeyCode::BackTab => self.cycle_board()?,
            KeyCode::Char('b') => {
                let cursor = self
                    .boards
                    .iter()
                    .position(|b| b.id == self.board.id)
                    .unwrap_or(0);
                self.mode = Mode::BoardSwitcher { cursor };
            }
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
                    self.open_card_board_move(card.key.clone(), ArchiveBack::Normal);
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

    pub(crate) fn on_input_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => self.return_from_input(),
            KeyCode::Enter => self.commit_input()?,
            KeyCode::Backspace => {
                if let Mode::Input { buffer, .. } = &mut self.mode {
                    buffer.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Mode::Input { buffer, .. } = &mut self.mode {
                    buffer.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_body_key(&mut self, key: KeyEvent) -> Result<()> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                if let Mode::BodyEdit { key, .. } = &self.mode {
                    self.mode = Mode::Detail {
                        key: key.clone(),
                        scroll: 0,
                    };
                }
            }
            KeyCode::Char('s') if ctrl => self.commit_body()?,
            KeyCode::Enter => {
                if let Mode::BodyEdit { editor, .. } = &mut self.mode {
                    editor.newline();
                }
            }
            KeyCode::Backspace => {
                if let Mode::BodyEdit { editor, .. } = &mut self.mode {
                    editor.backspace();
                }
            }
            KeyCode::Left => self.body_edit(|e| e.left()),
            KeyCode::Right => self.body_edit(|e| e.right()),
            KeyCode::Up => self.body_edit(|e| e.up()),
            KeyCode::Down => self.body_edit(|e| e.down()),
            KeyCode::Char(c) => {
                if let Mode::BodyEdit { editor, .. } = &mut self.mode {
                    editor.insert(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn body_edit(&mut self, f: impl FnOnce(&mut Editor)) {
        if let Mode::BodyEdit { editor, .. } = &mut self.mode {
            f(editor);
        }
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

    pub(crate) fn return_from_input(&mut self) {
        enum Back {
            Card(String),
            Columns,
            Normal,
        }
        let back = match &self.mode {
            Mode::Input { kind, .. } => match kind {
                InputKind::EditTitle { key, .. }
                | InputKind::EditAssignee { key, .. }
                | InputKind::EditDue { key, .. }
                | InputKind::CompleteWithNote { key, .. } => Back::Card(key.clone()),
                // Column edits return to the column manager.
                InputKind::NewColumn | InputKind::RenameColumn(_) => Back::Columns,
                InputKind::NewCard
                | InputKind::Filter
                | InputKind::NewBoard
                | InputKind::EditBoardContext => Back::Normal,
            },
            _ => Back::Normal,
        };
        self.mode = match back {
            Back::Card(key) if self.card_by_key(&key).is_some() => Mode::Detail { key, scroll: 0 },
            Back::Columns => Mode::ColumnManager,
            _ => Mode::Normal,
        };
    }

    pub(crate) fn on_archive_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::ArchiveConfirm {
            key: card_key,
            ref back,
        } = &self.mode
        else {
            return Ok(());
        };
        let card_key = card_key.clone();
        let back = *back;
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.archive_key(&card_key)?;
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.status = "archive cancelled".into();
                self.mode = self.archive_back_mode(&card_key, back);
            }
            _ => {}
        }
        Ok(())
    }
}
