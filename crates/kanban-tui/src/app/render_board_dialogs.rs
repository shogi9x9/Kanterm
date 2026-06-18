use super::App;
use crate::layout::centered_box;
use crate::theme::theme;
use kanban_core::{BoardColumnTemplate, PROTECTED_BOARD_SLUG};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_board_archive(&self, f: &mut Frame, board_name: &str) {
        let area = centered_box(f.area(), 64, 7);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(" archive board ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [message_area, hint_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Length(1)]).areas(inner);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Archive board '{board_name}'?")),
                Line::from(Span::styled(
                    "hidden from the board list; cards are kept (U to restore)",
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

    pub(crate) fn draw_board_switcher(&self, f: &mut Frame, cursor: usize) {
        let h = (self.boards.len() as u16 + 4).clamp(6, 22);
        let area = centered_box(f.area(), 54, h);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(" boards ");
        let items: Vec<ListItem> = self
            .boards
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let current = if b.id == self.board.id { "* " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(current, Style::default().fg(theme().success)),
                    Span::styled(
                        format!("{:>2}. ", i + 1),
                        Style::default().fg(theme().muted),
                    ),
                    Span::raw(b.name.clone()),
                    Span::styled(format!(" ({})", b.slug), Style::default().fg(theme().muted)),
                    Span::styled(
                        if b.agent_context.is_some() {
                            " context"
                        } else {
                            ""
                        },
                        Style::default().fg(theme().success),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme().selected_bg)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default();
        if !self.boards.is_empty() {
            state.select(Some(cursor.min(self.boards.len() - 1)));
        }
        f.render_stateful_widget(list, area, &mut state);
    }

    pub(crate) fn draw_board_template_picker(&self, f: &mut Frame, name: &str, cursor: usize) {
        let templates = BoardColumnTemplate::ALL;
        let h = (templates.len() as u16 + 6).clamp(8, 18);
        let area = centered_box(f.area(), 72, h);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(" board template ");
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
                Span::raw("new board: "),
                Span::styled(name.to_string(), Style::default().fg(theme().success)),
            ])),
            title_area,
        );

        let items: Vec<ListItem> = templates
            .iter()
            .enumerate()
            .map(|(i, template)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>2}. ", i + 1),
                        Style::default().fg(theme().muted),
                    ),
                    Span::styled(template.label(), Style::default().fg(theme().accent)),
                    Span::styled(
                        if *template == BoardColumnTemplate::DEFAULT_PROJECT {
                            " default"
                        } else {
                            ""
                        },
                        Style::default().fg(theme().success),
                    ),
                    Span::styled(
                        format!(" ({}) ", template.key()),
                        Style::default().fg(theme().muted),
                    ),
                    Span::raw(template.columns().join(" / ")),
                ]))
            })
            .collect();
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(theme().selected_bg)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default();
        state.select(Some(cursor.min(templates.len().saturating_sub(1))));
        f.render_stateful_widget(list, list_area, &mut state);
        f.render_widget(
            Paragraph::new(Span::styled(
                "Enter: create   j/k: select   Esc: cancel",
                Style::default().fg(theme().hint),
            )),
            hint_area,
        );
    }

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
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(theme().selected_bg)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
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
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(theme().selected_bg)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
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

    pub(crate) fn draw_board_unarchive(&self, f: &mut Frame, cursor: usize) {
        let archived = self.archived_boards().unwrap_or_default();
        let h = (archived.len() as u16 + 4).clamp(6, 20);
        let area = centered_box(f.area(), 50, h);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(" archived boards ");
        let items: Vec<ListItem> = archived
            .iter()
            .map(|b| ListItem::new(Line::from(format!("{} ({})", b.name, b.slug))))
            .collect();
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme().warning)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default();
        if !archived.is_empty() {
            state.select(Some(cursor.min(archived.len() - 1)));
        }
        f.render_stateful_widget(list, area, &mut state);
    }

    pub(crate) fn draw_board_delete(&self, f: &mut Frame, board_name: &str, confirm: &str) {
        let area = centered_box(f.area(), 64, 9);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().danger))
            .title(" delete board ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [message_area, input_area, hint_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .areas(inner);

        f.render_widget(
            Paragraph::new(Line::from(format!("delete board '{board_name}'?"))),
            message_area,
        );
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(theme().success)),
                Span::raw(confirm.to_string()),
            ])),
            input_area,
        );
        let max_cursor = input_area.width.saturating_sub(3) as usize;
        let cursor = confirm.chars().count().min(max_cursor) as u16;
        f.set_cursor_position(Position::new(input_area.x + 2 + cursor, input_area.y));
        f.render_widget(
            Paragraph::new(Span::styled(
                "type `delete` and Enter · Esc cancel",
                Style::default().fg(theme().hint),
            )),
            hint_area,
        );
    }
}
