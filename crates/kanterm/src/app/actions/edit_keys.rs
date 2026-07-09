use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::editor::Editor;
use crate::mode::{InputKind, Mode};

impl App {
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
}
