use super::App;
use crate::layout::centered_box;
use crate::theme::theme;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_archive_confirm(&self, f: &mut Frame, key: &str) {
        let title = self
            .card_by_key(key)
            .map(|c| c.title.as_str())
            .unwrap_or("(card not found)");
        let area = centered_box(f.area(), 64, 7);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(" archive card ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [message_area, hint_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Length(1)]).areas(inner);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Archive {key}?")),
                Line::from(Span::styled(
                    title.to_string(),
                    Style::default().fg(theme().success),
                )),
            ])
            .wrap(Wrap { trim: true }),
            message_area,
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                "y: archive   n/Esc: cancel",
                Style::default().fg(theme().hint),
            )),
            hint_area,
        );
    }
}
