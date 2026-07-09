use unicode_width::UnicodeWidthStr;

pub(crate) struct Editor {
    pub(crate) lines: Vec<String>,
    pub(crate) cy: usize,
    pub(crate) cx: usize,
}

impl Editor {
    pub(crate) fn new(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(|s| s.to_string()).collect()
        };
        let cy = lines.len() - 1;
        let cx = lines[cy].chars().count();
        Editor { lines, cy, cx }
    }

    pub(crate) fn text(&self) -> String {
        self.lines.join("\n")
    }

    fn byte_at(line: &str, char_idx: usize) -> usize {
        line.char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(line.len())
    }

    pub(crate) fn insert(&mut self, c: char) {
        let b = Self::byte_at(&self.lines[self.cy], self.cx);
        self.lines[self.cy].insert(b, c);
        self.cx += 1;
    }

    pub(crate) fn newline(&mut self) {
        let b = Self::byte_at(&self.lines[self.cy], self.cx);
        let rest = self.lines[self.cy].split_off(b);
        self.lines.insert(self.cy + 1, rest);
        self.cy += 1;
        self.cx = 0;
    }

    pub(crate) fn backspace(&mut self) {
        if self.cx > 0 {
            let b = Self::byte_at(&self.lines[self.cy], self.cx - 1);
            self.lines[self.cy].remove(b);
            self.cx -= 1;
        } else if self.cy > 0 {
            let cur = self.lines.remove(self.cy);
            self.cy -= 1;
            self.cx = self.lines[self.cy].chars().count();
            self.lines[self.cy].push_str(&cur);
        }
    }

    fn line_len(&self, y: usize) -> usize {
        self.lines[y].chars().count()
    }

    pub(crate) fn cursor_display_x(&self) -> usize {
        let line = &self.lines[self.cy];
        let byte = Self::byte_at(line, self.cx);
        UnicodeWidthStr::width(&line[..byte])
    }

    pub(crate) fn left(&mut self) {
        if self.cx > 0 {
            self.cx -= 1;
        } else if self.cy > 0 {
            self.cy -= 1;
            self.cx = self.line_len(self.cy);
        }
    }

    pub(crate) fn right(&mut self) {
        if self.cx < self.line_len(self.cy) {
            self.cx += 1;
        } else if self.cy + 1 < self.lines.len() {
            self.cy += 1;
            self.cx = 0;
        }
    }

    pub(crate) fn up(&mut self) {
        if self.cy > 0 {
            self.cy -= 1;
            self.cx = self.cx.min(self.line_len(self.cy));
        }
    }

    pub(crate) fn down(&mut self) {
        if self.cy + 1 < self.lines.len() {
            self.cy += 1;
            self.cx = self.cx.min(self.line_len(self.cy));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_display_x_counts_wide_glyphs() {
        let mut editor = Editor::new("ab漢字c");
        editor.cx = 4;
        assert_eq!(editor.cursor_display_x(), 6);
    }

    #[test]
    fn cursor_display_x_handles_mixed_lines() {
        let mut editor = Editor::new("abc\nあb");
        editor.cy = 1;
        editor.cx = 1;
        assert_eq!(editor.cursor_display_x(), 2);
        editor.cx = 2;
        assert_eq!(editor.cursor_display_x(), 3);
    }
}
