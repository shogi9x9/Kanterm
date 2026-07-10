use super::App;
use crate::layout::centered_box;
use crate::theme::{selection_style, theme};
use kanterm_core::format_date;
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_memory_browser(&self, f: &mut Frame, cursor: usize) {
        let memories = self.memories();
        let area = centered_box(
            f.area(),
            f.area().width.saturating_sub(8).min(100),
            f.area().height.saturating_sub(4).min(30),
        );
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(format!(" memories ({}) ", memories.len()));
        if memories.is_empty() {
            let inner = block.inner(area);
            f.render_widget(block, area);
            f.render_widget(
                Paragraph::new(Span::styled(
                    "no memories yet - agents record them via record_memory",
                    Style::default().fg(theme().hint),
                )),
                inner,
            );
            return;
        }
        let items: Vec<ListItem> = memories
            .iter()
            .map(|m| {
                let mut spans = vec![
                    Span::styled(
                        format!("{:<5} ", m.key),
                        Style::default().fg(theme().warning),
                    ),
                    Span::styled(
                        format!("[{:<8}] ", m.kind),
                        Style::default().fg(theme().success),
                    ),
                    Span::styled(
                        format!("{} ", format_date(m.created_at)),
                        Style::default().fg(theme().muted),
                    ),
                    Span::raw(m.title.clone()),
                ];
                if let Some(ck) = &m.card_key {
                    spans.push(Span::styled(
                        format!("  ({ck})"),
                        Style::default().fg(theme().accent),
                    ));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        let list = List::new(items)
            .block(block)
            .highlight_style(selection_style())
            .highlight_symbol(theme().selection_symbol);
        let mut state = ListState::default();
        state.select(Some(cursor.min(memories.len() - 1)));
        f.render_stateful_widget(list, area, &mut state);
    }

    pub(crate) fn draw_memory_detail(&self, f: &mut Frame, key: &str) {
        let Some(m) = self.store.memory_by_key(key).ok().flatten() else {
            return;
        };
        let area = centered_box(
            f.area(),
            f.area().width.saturating_sub(10).min(90),
            f.area().height.saturating_sub(6).min(26),
        );
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(format!(" {} [{}] ", m.key, m.kind))
            .padding(Padding::new(1, 1, 0, 0));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut meta = format!("recorded {}", format_date(m.created_at));
        if let Some(ck) = &m.card_key {
            meta.push_str(&format!("   card: {ck}"));
        }
        let mut lines = vec![
            Line::from(Span::styled(
                m.title.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(meta, Style::default().fg(theme().muted))),
            Line::from(""),
        ];
        lines.extend(m.body.lines().map(|l| Line::from(l.to_string())));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    pub(crate) fn draw_memory_archive(&self, f: &mut Frame, key: &str) {
        let title = self
            .store
            .memory_by_key(key)
            .ok()
            .flatten()
            .map(|m| m.title)
            .unwrap_or_else(|| "(memory not found)".into());
        let area = centered_box(f.area(), 64, 7);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(" archive memory ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        let [message_area, hint_area] =
            ratatui::layout::Layout::vertical([Constraint::Length(3), Constraint::Length(1)])
                .areas(inner);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Archive {key}?")),
                Line::from(Span::styled(title, Style::default().fg(theme().success))),
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
