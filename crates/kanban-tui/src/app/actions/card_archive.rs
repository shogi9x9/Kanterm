use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::Mode;

impl App {
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
