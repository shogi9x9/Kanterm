use std::io::stdout;

use anyhow::Result;
use crossterm::{clipboard::CopyToClipboard, execute};

pub(crate) trait ClipboardWriter {
    fn write(&mut self, content: &str) -> Result<()>;
}

pub(crate) struct Osc52Clipboard;

impl ClipboardWriter for Osc52Clipboard {
    fn write(&mut self, content: &str) -> Result<()> {
        execute!(stdout(), CopyToClipboard::to_clipboard_from(content))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingClipboard(String);

    impl ClipboardWriter for RecordingClipboard {
        fn write(&mut self, content: &str) -> Result<()> {
            self.0 = content.to_string();
            Ok(())
        }
    }

    #[test]
    fn clipboard_contract_is_write_only() {
        let mut clipboard = RecordingClipboard::default();
        clipboard.write("work packet").unwrap();
        assert_eq!(clipboard.0, "work packet");
    }
}
