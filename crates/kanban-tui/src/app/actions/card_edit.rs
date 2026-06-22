use anyhow::Result;
use kanban_core::{BoardColumnTemplate, CardPatch};

use crate::app::{App, ACTOR};
use crate::mode::{InputKind, Mode};

impl App {
    pub(crate) fn commit_input(&mut self) -> Result<()> {
        let mode = std::mem::replace(&mut self.mode, Mode::Normal);
        let Mode::Input { kind, buffer } = mode else {
            return Ok(());
        };
        let text = buffer.trim().to_string();
        match kind {
            InputKind::NewCard => {
                if text.is_empty() {
                    self.status = "cancelled (empty)".into();
                    return Ok(());
                }
                let Some(col) = self.columns.get(self.focus).map(|c| c.name.clone()) else {
                    return Ok(());
                };
                let card = self
                    .store
                    .create_card(&self.board.id, Some(&col), &text, "", ACTOR)?;
                let key = card.key.clone();
                self.reload()?;
                self.select_key(&key);
                self.status = format!("created {key}");
            }
            InputKind::EditTitle {
                key,
                expected_updated_at,
            } => {
                if !text.is_empty() {
                    let patch = CardPatch {
                        title: Some(text),
                        expected_updated_at: Some(expected_updated_at),
                        ..Default::default()
                    };
                    match self.store.update_card(&self.board.id, &key, &patch, ACTOR) {
                        Ok(_) => {
                            self.reload()?;
                            self.select_key(&key);
                            self.status = format!("updated {key}");
                        }
                        Err(e) if e.to_string().contains("stale update") => {
                            self.reload()?;
                            self.status = format!("stale edit on {key}; reloaded latest card");
                        }
                        Err(e) => return Err(e),
                    }
                }
                self.mode = Mode::Detail { key, scroll: 0 };
            }
            InputKind::EditAssignee {
                key,
                expected_updated_at,
            } => {
                let patch = CardPatch {
                    assignee: Some(text),
                    expected_updated_at: Some(expected_updated_at),
                    ..Default::default()
                };
                match self.store.update_card(&self.board.id, &key, &patch, ACTOR) {
                    Ok(_) => {
                        self.reload()?;
                        self.status = format!("assignee set on {key}");
                    }
                    Err(e) if e.to_string().contains("stale update") => {
                        self.reload()?;
                        self.status = format!("stale edit on {key}; reloaded latest card");
                    }
                    Err(e) => return Err(e),
                }
                self.mode = Mode::Detail { key, scroll: 0 };
            }
            InputKind::EditDue {
                key,
                expected_updated_at,
            } => {
                // `text` is "" to clear, or "YYYY-MM-DD". Invalid input is
                // reported and the card is left unchanged.
                let patch = CardPatch {
                    due: Some(text),
                    expected_updated_at: Some(expected_updated_at),
                    ..Default::default()
                };
                match self.store.update_card(&self.board.id, &key, &patch, ACTOR) {
                    Ok(_) => {
                        self.reload()?;
                        self.status = format!("due updated on {key}");
                    }
                    Err(e) if e.to_string().contains("stale update") => {
                        self.reload()?;
                        self.status = format!("stale edit on {key}; reloaded latest card");
                    }
                    Err(e) => self.status = format!("bad date: {e}"),
                }
                self.mode = Mode::Detail { key, scroll: 0 };
            }
            InputKind::CompleteWithNote {
                key,
                expected_updated_at,
            } => {
                let mut body = self
                    .card_by_key(&key)
                    .map(|c| c.body.clone())
                    .unwrap_or_default();
                if !text.is_empty() {
                    if !body.is_empty() {
                        body.push('\n');
                        body.push('\n');
                    }
                    body.push_str(&format!("[completion note] {text}"));
                }
                let patch = CardPatch {
                    archived: Some(true),
                    body: Some(body),
                    agent_state: Some("done".into()),
                    next_action: Some(String::new()),
                    blocked_reason: Some(String::new()),
                    handoff_note: Some(String::new()),
                    expected_updated_at: Some(expected_updated_at),
                    ..Default::default()
                };
                match self.store.update_card(&self.board.id, &key, &patch, ACTOR) {
                    Ok(_) => {
                        self.reload()?;
                        self.status = if text.is_empty() {
                            format!("completed {key}")
                        } else {
                            format!("completed {key} with note")
                        };
                    }
                    Err(e) if e.to_string().contains("stale update") => {
                        self.reload()?;
                        self.status = format!("stale complete on {key}; reloaded latest card");
                    }
                    Err(e) => return Err(e),
                }
                self.mode = Mode::Normal;
            }
            InputKind::Filter => {
                self.filter = if text.is_empty() { None } else { Some(text) };
                self.reload()?;
                self.status = match &self.filter {
                    Some(f) => format!("filter: {f}"),
                    None => "filter cleared".into(),
                };
            }
            InputKind::NewBoard => {
                if text.is_empty() {
                    self.status = "cancelled (empty)".into();
                    return Ok(());
                }
                self.mode = Mode::BoardTemplatePicker {
                    name: text,
                    cursor: BoardColumnTemplate::default_index(),
                };
            }
            InputKind::EditBoardContext => {
                let updated = self
                    .store
                    .update_board_agent_context(&self.board.id, Some(&text))?;
                self.reload()?;
                self.status = if updated.agent_context.is_some() {
                    "board agent context updated".into()
                } else {
                    "board agent context cleared".into()
                };
            }
            InputKind::NewColumn => {
                if text.is_empty() {
                    self.status = "cancelled (empty)".into();
                } else {
                    match self.store.add_column(&self.board.id, &text) {
                        Ok(c) => {
                            self.refresh_columns()?;
                            if let Some(pos) = self.columns.iter().position(|x| x.id == c.id) {
                                self.col_cursor = pos;
                            }
                            self.status = format!("added column {}", c.name);
                        }
                        Err(e) => self.status = format!("error: {e}"),
                    }
                }
                self.mode = Mode::ColumnManager;
            }
            InputKind::RenameColumn(id) => {
                if !text.is_empty() {
                    match self.store.rename_column(&id, &text) {
                        Ok(()) => {
                            self.refresh_columns()?;
                            self.status = format!("renamed column to {text}");
                        }
                        Err(e) => self.status = format!("error: {e}"),
                    }
                }
                self.mode = Mode::ColumnManager;
            }
        }
        Ok(())
    }

    pub(crate) fn commit_body(&mut self) -> Result<()> {
        if let Mode::BodyEdit {
            key,
            editor,
            expected_updated_at,
        } = &self.mode
        {
            let key = key.clone();
            let patch = CardPatch {
                body: Some(editor.text()),
                expected_updated_at: Some(*expected_updated_at),
                ..Default::default()
            };
            match self.store.update_card(&self.board.id, &key, &patch, ACTOR) {
                Ok(_) => {
                    self.reload()?;
                    self.status = format!("saved body of {key}");
                }
                Err(e) if e.to_string().contains("stale update") => {
                    self.reload()?;
                    self.status = format!("stale body edit on {key}; reloaded latest card");
                }
                Err(e) => return Err(e),
            }
            self.mode = Mode::Detail { key, scroll: 0 };
        }
        Ok(())
    }
}
