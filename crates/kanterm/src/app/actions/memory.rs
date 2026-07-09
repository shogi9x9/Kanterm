use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use kanterm_core::Memory;

use crate::app::App;
use crate::mode::Mode;

impl App {
    /// Active memories, newest first (the browser's working set).
    pub(crate) fn memories(&self) -> Vec<Memory> {
        self.store
            .recall_memories(None, None, None, 500, false)
            .unwrap_or_default()
    }

    pub(crate) fn on_memory_browser_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::MemoryBrowser { ref cursor } = &self.mode else {
            return Ok(());
        };
        let cursor = *cursor;
        let memories = self.memories();
        let cursor = cursor.min(memories.len().saturating_sub(1));
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => self.mode = Mode::Normal,
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::MemoryBrowser {
                    cursor: (cursor + 1).min(memories.len().saturating_sub(1)),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::MemoryBrowser {
                    cursor: cursor.saturating_sub(1),
                };
            }
            KeyCode::Enter => {
                if let Some(m) = memories.get(cursor) {
                    self.mode = Mode::MemoryDetail {
                        key: m.key.clone(),
                        cursor,
                    };
                }
            }
            KeyCode::Char('d') => {
                if let Some(m) = memories.get(cursor) {
                    self.mode = Mode::MemoryArchive {
                        key: m.key.clone(),
                        cursor,
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_memory_detail_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::MemoryDetail { ref cursor, .. } = &self.mode else {
            return Ok(());
        };
        let cursor = *cursor;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::MemoryBrowser { cursor },
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn on_memory_archive_key(&mut self, key: KeyEvent) -> Result<()> {
        let Mode::MemoryArchive {
            key: mem_key,
            ref cursor,
        } = &self.mode
        else {
            return Ok(());
        };
        let mem_key = mem_key.clone();
        let cursor = *cursor;
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.store.update_memory(
                    &mem_key,
                    &kanterm_core::MemoryPatch {
                        archived: Some(true),
                        ..Default::default()
                    },
                )?;
                self.status = format!("archived memory {mem_key}");
                self.mode = Mode::MemoryBrowser { cursor };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.status = "cancelled".into();
                self.mode = Mode::MemoryBrowser { cursor };
            }
            _ => {}
        }
        Ok(())
    }
}
