use anyhow::Result;
use kanterm_core::{priority_badge, CardPatch, PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_NORMAL};

use crate::app::{App, ACTOR};
use crate::mode::{ArchiveBack, Mode};

impl App {
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
