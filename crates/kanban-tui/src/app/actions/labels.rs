use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use kanban_core::{CardPatch, Label};

use crate::app::{App, ACTOR, LABEL_RECENCY_MS};
use crate::mode::Mode;

impl App {
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
}
