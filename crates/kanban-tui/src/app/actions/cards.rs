use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use kanban_core::{
    priority_badge, BoardColumnTemplate, CardPatch, Label, PRIORITY_HIGH, PRIORITY_LOW,
    PRIORITY_NORMAL,
};

use crate::app::{App, ACTOR, LABEL_RECENCY_MS};
use crate::mode::{ArchiveBack, InputKind, Mode};

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
                    body.push_str(&format!("[完了メモ] {text}"));
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

    // -- label picker --------------------------------------------------------

    pub(crate) fn open_label_picker(&mut self, key: String) -> Result<()> {
        let candidates = self.label_candidates(&key)?;
        self.mode = Mode::LabelPicker {
            key,
            input: String::new(),
            cursor: 0,
            candidates,
        };
        Ok(())
    }

    /// Recently used labels plus any already on the given card (so stale ones
    /// can still be removed), de-duplicated by name.
    pub(crate) fn label_candidates(&self, card_key: &str) -> Result<Vec<Label>> {
        let mut out = self.store.recent_labels(LABEL_RECENCY_MS)?;
        if let Some(card) = self.card_by_key(card_key) {
            if let Some(on_card) = self.labels.get(&card.id) {
                for l in on_card {
                    if !out.iter().any(|c| c.name == l.name) {
                        out.push(l.clone());
                    }
                }
            }
        }
        Ok(out)
    }

    pub(crate) fn card_has_label(&self, card_key: &str, label_name: &str) -> bool {
        self.card_by_key(card_key)
            .and_then(|c| self.labels.get(&c.id))
            .map(|ls| ls.iter().any(|l| l.name == label_name))
            .unwrap_or(false)
    }

    pub(crate) fn on_label_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::LabelPicker {
            key: card_key,
            input,
            ref mut cursor,
            ref mut candidates,
        } = &mut self.mode
        else {
            return Ok(());
        };
        match key.code {
            KeyCode::Esc => {
                let key = card_key.clone();
                self.mode = Mode::Detail { key, scroll: 0 };
            }
            KeyCode::Up => *cursor = cursor.saturating_sub(1),
            KeyCode::Down => {
                if !candidates.is_empty() {
                    *cursor = (*cursor + 1).min(candidates.len() - 1);
                }
            }
            KeyCode::Char(' ') => {
                // Toggle the highlighted candidate on/off the card.
                if let Some(label) = candidates.get(*cursor) {
                    let (card_key, name) = (card_key.clone(), label.name.clone());
                    self.toggle_label(&card_key, &name)?;
                }
            }
            KeyCode::Enter => {
                // Attach the typed label (created on demand), then clear input.
                let name = input.trim().to_string();
                if !name.is_empty() {
                    let card_key = card_key.clone();
                    if !self.card_has_label(&card_key, &name) {
                        let patch = CardPatch {
                            add_labels: Some(vec![name]),
                            ..Default::default()
                        };
                        self.store
                            .update_card(&self.board.id, &card_key, &patch, ACTOR)?;
                        self.reload()?;
                    }
                    self.refresh_label_picker()?;
                }
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) => input.push(c),
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn toggle_label(&mut self, card_key: &str, name: &str) -> Result<()> {
        let patch = if self.card_has_label(card_key, name) {
            CardPatch {
                remove_labels: Some(vec![name.to_string()]),
                ..Default::default()
            }
        } else {
            CardPatch {
                add_labels: Some(vec![name.to_string()]),
                ..Default::default()
            }
        };
        self.store
            .update_card(&self.board.id, card_key, &patch, ACTOR)?;
        self.reload()?;
        self.refresh_label_picker()
    }

    /// Recompute the candidate list while keeping the cursor and input field,
    /// clearing the input (called after adding a brand-new label).
    pub(crate) fn refresh_label_picker(&mut self) -> Result<()> {
        let card_key = match &self.mode {
            Mode::LabelPicker { key, .. } => key.clone(),
            _ => return Ok(()),
        };
        let fresh = self.label_candidates(&card_key)?;
        if let Mode::LabelPicker {
            candidates,
            ref mut cursor,
            input,
            ..
        } = &mut self.mode
        {
            *candidates = fresh;
            if *cursor >= candidates.len() {
                *cursor = candidates.len().saturating_sub(1);
            }
            input.clear();
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

    pub(crate) fn focus_delta(&mut self, delta: i32) {
        if self.columns.is_empty() {
            return;
        }
        let max = self.columns.len() as i32 - 1;
        self.focus = (self.focus as i32 + delta).clamp(0, max) as usize;
    }

    pub(crate) fn cursor_delta(&mut self, delta: i32) {
        let len = self.column_cards(self.focus).len();
        if len == 0 {
            return;
        }
        let max = len as i32 - 1;
        let cur = self.cursors[self.focus] as i32;
        self.cursors[self.focus] = (cur + delta).clamp(0, max) as usize;
    }

    pub(crate) fn move_card(&mut self, delta: i32) -> Result<()> {
        let target = self.focus as i32 + delta;
        if target < 0 || target >= self.columns.len() as i32 {
            return Ok(());
        }
        let Some(card) = self.selected_card() else {
            return Ok(());
        };
        let key = card.key.clone();
        let dest = self.columns[target as usize].name.clone();
        self.store.move_card(&self.board.id, &key, &dest, ACTOR)?;
        self.reload()?;
        self.select_key(&key);
        self.status = format!("moved {key} -> {dest}");
        Ok(())
    }

    pub(crate) fn reorder(&mut self, dir: i32) -> Result<()> {
        let Some(card) = self.selected_card() else {
            return Ok(());
        };
        let key = card.key.clone();
        self.store.reorder_card(&self.board.id, &key, dir)?;
        self.reload()?;
        self.select_key(&key);
        Ok(())
    }

    pub(crate) fn undo_last_card_update(&mut self) -> Result<()> {
        match self.store.undo_last_card_update(&self.board.id, ACTOR)? {
            Some(card) => {
                let key = card.key.clone();
                self.reload()?;
                if self.board.id != card.board_id {
                    if let Some(board) = self.boards.iter().find(|b| b.id == card.board_id).cloned()
                    {
                        self.switch_board(board)?;
                    }
                }
                self.select_key(&key);
                self.status = format!("undid last update on {key}");
            }
            None => {
                self.status = "nothing to undo".into();
            }
        }
        Ok(())
    }

    pub(crate) fn cycle_priority(&mut self) -> Result<()> {
        if let Some(c) = self.selected_card() {
            let key = c.key.clone();
            self.cycle_priority_key(&key)?;
        }
        Ok(())
    }

    pub(crate) fn cycle_priority_key(&mut self, key: &str) -> Result<()> {
        let Some(c) = self.card_by_key(key) else {
            return Ok(());
        };
        let next = match c.priority {
            PRIORITY_LOW => PRIORITY_NORMAL,
            PRIORITY_NORMAL => PRIORITY_HIGH,
            _ => PRIORITY_LOW,
        };
        let patch = CardPatch {
            priority: Some(next),
            ..Default::default()
        };
        self.store.update_card(&self.board.id, key, &patch, ACTOR)?;
        self.reload()?;
        self.status = format!("{key} priority -> {}", priority_badge(next));
        Ok(())
    }

    pub(crate) fn prompt_archive_selected(&mut self) {
        if let Some(c) = self.selected_card() {
            let key = c.key.clone();
            self.prompt_archive_key(key, ArchiveBack::Normal);
        }
    }

    pub(crate) fn prompt_archive_key(&mut self, key: String, back: ArchiveBack) {
        self.mode = Mode::ArchiveConfirm { key, back };
        self.status = "archive? y/n".into();
    }

    pub(crate) fn archive_back_mode(&self, key: &str, back: ArchiveBack) -> Mode {
        match back {
            ArchiveBack::Detail if self.card_by_key(key).is_some() => Mode::Detail {
                key: key.to_string(),
                scroll: 0,
            },
            _ => Mode::Normal,
        }
    }

    pub(crate) fn archive_key(&mut self, key: &str) -> Result<()> {
        let patch = CardPatch {
            archived: Some(true),
            ..Default::default()
        };
        self.store.update_card(&self.board.id, key, &patch, ACTOR)?;
        self.reload()?;
        self.status = format!("archived {key}");
        Ok(())
    }
}
