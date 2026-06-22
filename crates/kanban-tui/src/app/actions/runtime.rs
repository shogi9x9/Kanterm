use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::app::App;

impl App {
    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // event::poll blocks efficiently (timed wait, no busy loop). External DB
        // writes do not wake the terminal event stream, so keep this short enough
        // that MCP/agent edits feel live while still doing only one cheap PRAGMA
        // per idle tick.
        const POLL: std::time::Duration = std::time::Duration::from_millis(150);
        terminal.draw(|f| self.draw(f))?;
        while !self.should_quit {
            if event::poll(POLL)? {
                let synced = self.sync_external()?;
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.on_key(key)?;
                        terminal.draw(|f| self.draw(f))?;
                    }
                    Event::Resize(_, _) => {
                        terminal.draw(|f| self.draw(f))?;
                    }
                    _ if synced => {
                        terminal.draw(|f| self.draw(f))?;
                    }
                    _ => {}
                }
            } else if self.sync_external()? {
                terminal.draw(|f| self.draw(f))?;
            }
        }
        self.save_ui_state();
        Ok(())
    }

    /// If another connection (the MCP server) has committed since we last looked,
    /// reload and report true so the caller repaints. Cheap: one PRAGMA, and the
    /// table reload runs only when the version actually moved.
    pub(crate) fn sync_external(&mut self) -> Result<bool> {
        let v = self.store.data_version().unwrap_or(self.data_version);
        if v == self.data_version {
            return Ok(false);
        }
        self.data_version = v;
        let previous_status = self.status.clone();
        self.resync_external()?;
        if self.status == previous_status {
            self.status = "↻ synced external change".into();
        }
        Ok(true)
    }
}
