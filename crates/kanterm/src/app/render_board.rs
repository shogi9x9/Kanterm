use super::App;
use crate::theme::{hex_to_color, priority_span, selection_style, theme};
use kanterm_core::{card_is_stale, format_date, today_start_ms, HumanIntervention};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_header(&self, f: &mut Frame, area: Rect) {
        let pos = self
            .boards
            .iter()
            .position(|b| b.id == self.board.id)
            .unwrap_or(0);
        let total = self.boards.len().max(1);
        let mut spans = vec![
            Span::styled(
                " KANTERM ",
                Style::default()
                    .fg(theme().accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│", Style::default().fg(theme().muted)),
            Span::styled(" board ", Style::default().fg(theme().muted)),
            Span::styled("< ", Style::default().fg(theme().help)),
            Span::styled(format!(" {} ", self.board.name), selection_style()),
            Span::styled(" >", Style::default().fg(theme().help)),
            Span::styled(
                format!(" {}/{} ", pos + 1, total),
                Style::default().fg(theme().muted),
            ),
        ];
        if self.board.agent_context.is_some() {
            spans.push(Span::styled(
                "  ● context",
                Style::default().fg(theme().success),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    pub(crate) fn draw_column(&self, f: &mut Frame, col: usize, area: Rect) {
        let focused = col == self.focus;
        let cards = self.column_cards(col);
        let border_color = if focused {
            theme().accent
        } else {
            theme().muted
        };
        let title = Line::from(vec![
            Span::styled(
                format!(" {} ", self.columns[col].name),
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("· {} ", cards.len()),
                Style::default().fg(theme().muted),
            ),
        ]);
        let horizontal_padding = u16::from(area.width >= 24);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(horizontal_padding, horizontal_padding, 0, 0))
            .title(title);

        let items: Vec<ListItem> = cards
            .iter()
            .map(|c| {
                let marker = priority_span(c.priority);
                let key = Span::styled(format!("{} ", c.key), Style::default().fg(theme().muted));
                let mut spans = vec![marker, Span::raw(" "), key, Span::raw(c.title.clone())];
                if let Some(ms) = c.due_date {
                    let overdue = ms < today_start_ms();
                    spans.push(Span::styled(
                        format!(" ⏰{}", format_date(ms)),
                        Style::default().fg(if overdue {
                            theme().danger
                        } else {
                            theme().muted
                        }),
                    ));
                }
                if let Some(ls) = self.labels.get(&c.id) {
                    for l in ls {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(
                            format!("●{}", l.name),
                            Style::default().fg(hex_to_color(&l.color)),
                        ));
                    }
                }
                if card_is_stale(c) {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        "[stale]",
                        Style::default().fg(theme().warning),
                    ));
                }
                if let Some(human) = human_gate(c.human_intervention.as_deref()) {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("[H:{human}]"),
                        Style::default()
                            .fg(theme().warning)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();

        let mut list = List::new(items).block(block);
        if focused {
            list = list
                .highlight_style(selection_style())
                .highlight_symbol(theme().selection_symbol);
        }

        let mut state = ListState::default();
        if focused && !cards.is_empty() {
            state.select(Some(self.cursors[col].min(cards.len() - 1)));
        }
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn human_gate(value: Option<&str>) -> Option<&str> {
    value
        .and_then(|v| HumanIntervention::parse(v).ok().flatten())
        .map(HumanIntervention::as_str)
}

#[cfg(test)]
mod tests {
    use super::human_gate;

    #[test]
    fn human_gate_labels_only_intervention_states() {
        assert_eq!(human_gate(Some("review")), Some("review"));
        assert_eq!(human_gate(Some("decision")), Some("decision"));
        assert_eq!(human_gate(Some("execution")), Some("execution"));
        assert_eq!(human_gate(Some("none")), None);
        assert_eq!(human_gate(None), None);
    }
}
