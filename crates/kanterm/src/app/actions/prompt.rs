use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{Mode, ViewBack};

impl App {
    pub(crate) fn open_execution_prompt(&mut self, key: String) {
        match self.store.execution_prompt(&self.board.id, &key) {
            Ok(prompt) => {
                self.status.clear();
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: 0,
                };
            }
            Err(error) => {
                self.status = format!("cannot build execution prompt: {error}");
            }
        }
    }

    pub(crate) fn open_board_execution_prompt(&mut self, back: ViewBack) {
        match self.store.board_execution_prompt(&self.board.id) {
            Ok(prompt) => {
                self.status.clear();
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: 0,
                    back,
                };
            }
            Err(error) => {
                self.status = format!("cannot build board execution prompt: {error}");
            }
        }
    }

    pub(crate) fn on_execution_prompt_key(&mut self, event: KeyEvent) {
        let Mode::ExecutionPrompt {
            key,
            prompt,
            scroll,
        } = &self.mode
        else {
            return;
        };
        let key = key.clone();
        let prompt = prompt.clone();
        let scroll = *scroll;
        match event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Detail { key, scroll: 0 };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: scroll.saturating_add(1),
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: scroll.saturating_sub(1),
                };
            }
            KeyCode::PageDown => {
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: scroll.saturating_add(10),
                };
            }
            KeyCode::PageUp => {
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: scroll.saturating_sub(10),
                };
            }
            KeyCode::Home => {
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: 0,
                };
            }
            KeyCode::End => {
                self.mode = Mode::ExecutionPrompt {
                    key,
                    prompt,
                    scroll: u16::MAX,
                };
            }
            KeyCode::Enter | KeyCode::Char('c') | KeyCode::Char('y') => {
                self.copy_prompt(&key, &prompt);
            }
            _ => {}
        }
    }

    pub(crate) fn on_board_execution_prompt_key(&mut self, event: KeyEvent) {
        let Mode::BoardExecutionPrompt {
            prompt,
            scroll,
            back,
        } = &self.mode
        else {
            return;
        };
        let prompt = prompt.clone();
        let scroll = *scroll;
        let back = *back;
        match event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = back.return_mode(),
            KeyCode::Down | KeyCode::Char('j') => {
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: scroll.saturating_add(1),
                    back,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: scroll.saturating_sub(1),
                    back,
                };
            }
            KeyCode::PageDown => {
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: scroll.saturating_add(10),
                    back,
                };
            }
            KeyCode::PageUp => {
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: scroll.saturating_sub(10),
                    back,
                };
            }
            KeyCode::Home => {
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: 0,
                    back,
                };
            }
            KeyCode::End => {
                self.mode = Mode::BoardExecutionPrompt {
                    prompt,
                    scroll: u16::MAX,
                    back,
                };
            }
            KeyCode::Enter | KeyCode::Char('c') | KeyCode::Char('y') => {
                let subject = self.board.slug.clone();
                self.copy_prompt(&subject, &prompt);
            }
            _ => {}
        }
    }

    fn copy_prompt(&mut self, subject: &str, prompt: &str) {
        self.status = match self.clipboard.write(prompt) {
            Ok(()) => format!(
                "sent {} bytes for {subject} to the terminal clipboard",
                prompt.len()
            ),
            Err(error) => format!("clipboard request failed: {error}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use crossterm::event::KeyModifiers;
    use kanterm_core::Store;
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;
    use crate::clipboard::ClipboardWriter;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[derive(Default)]
    struct RecordingClipboard {
        content: String,
    }

    impl ClipboardWriter for RecordingClipboard {
        fn write(&mut self, content: &str) -> Result<()> {
            self.content = content.to_string();
            Ok(())
        }
    }

    #[test]
    fn card_prompt_previews_before_copy_and_returns_to_detail() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "Copy me", "description", "test")
            .unwrap();
        let mut app = App::new(store, board).unwrap();
        app.clipboard = Box::new(RecordingClipboard::default());
        app.mode = Mode::Detail {
            key: card.key.clone(),
            scroll: 0,
        };

        app.on_detail_key(key(KeyCode::Char('C'))).unwrap();
        assert!(matches!(app.mode, Mode::ExecutionPrompt { .. }));
        app.on_execution_prompt_key(key(KeyCode::Enter));
        assert!(app.status.contains(&card.key));
        let mut terminal = Terminal::new(TestBackend::new(120, 1)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                app.draw_status(frame, area);
            })
            .unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("sent"), "got: {rendered}");
        app.on_execution_prompt_key(key(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Detail { .. }));
    }

    #[test]
    fn board_prompt_preserves_dashboard_return_view() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        store
            .create_card(&board.id, None, "Board work", "", "test")
            .unwrap();
        let mut app = App::new(store, board).unwrap();
        let state = crate::mode::ExecutionDashboardState::new(
            crate::mode::ExecutionDashboardView::Timeline,
            0,
            0,
        );
        app.mode = Mode::ExecutionDashboard(state);

        app.on_execution_dashboard_key(key(KeyCode::Char('C')))
            .unwrap();
        assert!(matches!(app.mode, Mode::BoardExecutionPrompt { .. }));
        app.on_board_execution_prompt_key(key(KeyCode::Esc));
        assert!(matches!(
            app.mode,
            Mode::ExecutionDashboard(restored) if restored == state
        ));
    }
}
