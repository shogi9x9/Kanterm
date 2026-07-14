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
                self.mode = back.return_mode(None);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.status = "archive cancelled".into();
                let detail_key = self.card_by_key(&card_key).map(|_| card_key);
                self.mode = back.return_mode(detail_key);
            }
            _ => {}
        }
        Ok(())
    }
}
