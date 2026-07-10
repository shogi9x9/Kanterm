use super::App;
use crate::layout::centered_box;
use crate::theme::{selection_style, theme};
use kanterm_core::PROTECTED_BOARD_SLUG;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_card_board_move(&self, f: &mut Frame, key: &str, cursor: usize) {
        let destinations = self.card_move_destinations();
        let h = (destinations.len() as u16 + 5).clamp(7, 22);
        let area = centered_box(f.area(), 58, h);
        f.render_widget(Clear, area);
        let from_backlog = self.board.slug == PROTECTED_BOARD_SLUG;
        let title = if from_backlog {
            " send to project board "
        } else {
            " move card to board "
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [title_area, list_area, hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(key.to_string(), Style::default().fg(theme().success)),
                Span::raw(if from_backlog {
                    " project destination"
                } else {
                    " destination"
                }),
            ])),
            title_area,
        );

        let items: Vec<ListItem> = destinations
            .iter()
            .enumerate()
            .map(|(i, b)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>2}. ", i + 1),
                        Style::default().fg(theme().muted),
                    ),
                    Span::raw(b.name.clone()),
                    Span::styled(format!(" ({})", b.slug), Style::default().fg(theme().muted)),
                ]))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(selection_style())
            .highlight_symbol(theme().selection_symbol);
        let mut state = ListState::default();
        if !destinations.is_empty() {
            state.select(Some(cursor.min(destinations.len() - 1)));
        }
        f.render_stateful_widget(list, list_area, &mut state);
        f.render_widget(
            Paragraph::new(Span::styled(
                "Enter: choose column   j/k: select   M/Esc: cancel",
                Style::default().fg(theme().hint),
            )),
            hint_area,
        );
    }

    pub(crate) fn draw_card_column_move(
        &self,
        f: &mut Frame,
        key: &str,
        board_id: &str,
        board_name: &str,
        cursor: usize,
    ) {
        let columns = self.store.columns(board_id).unwrap_or_default();
        let h = (columns.len() as u16 + 5).clamp(7, 22);
        let area = centered_box(f.area(), 58, h);
        f.render_widget(Clear, area);
        let from_backlog = self.board.slug == PROTECTED_BOARD_SLUG;
        let title = if from_backlog {
            " send to project column "
        } else {
            " move card to column "
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [title_area, list_area, hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(key.to_string(), Style::default().fg(theme().success)),
                Span::raw(" -> "),
                Span::styled(board_name.to_string(), Style::default().fg(theme().accent)),
            ])),
            title_area,
        );

        let items: Vec<ListItem> = columns
            .iter()
            .enumerate()
            .map(|(i, c)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>2}. ", i + 1),
                        Style::default().fg(theme().muted),
                    ),
                    Span::raw(c.name.clone()),
                ]))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(selection_style())
            .highlight_symbol(theme().selection_symbol);
        let mut state = ListState::default();
        if !columns.is_empty() {
            state.select(Some(cursor.min(columns.len() - 1)));
        }
        f.render_stateful_widget(list, list_area, &mut state);
        f.render_widget(
            Paragraph::new(Span::styled(
                if from_backlog {
                    "Enter: send   j/k: select   b: boards   M/Esc: cancel"
                } else {
                    "Enter: move   j/k: select   b: boards   M/Esc: cancel"
                },
                Style::default().fg(theme().hint),
            )),
            hint_area,
        );
    }
}
