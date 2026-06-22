use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{InputKind, Mode};

impl App {
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
}
