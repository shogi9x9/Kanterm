use super::App;
use crate::layout::centered;
use crate::theme::theme;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_execution_prompt(
        &self,
        f: &mut Frame,
        key: &str,
        prompt: &str,
        scroll: u16,
    ) {
        self.draw_prompt_preview(f, &format!("execution prompt: {key}"), prompt, scroll);
    }

    pub(crate) fn draw_board_execution_prompt(&self, f: &mut Frame, prompt: &str, scroll: u16) {
        self.draw_prompt_preview(
            f,
            &format!("board execution prompt: {}", self.board.name),
            prompt,
            scroll,
        );
    }

    fn draw_prompt_preview(&self, f: &mut Frame, title: &str, prompt: &str, scroll: u16) {
        let area = centered(f.area(), 84, 84);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(format!(" {title} · {} bytes ", prompt.len()));
        f.render_widget(
            Paragraph::new(prompt)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            area,
        );
    }
}
