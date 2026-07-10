use super::App;
use crate::app::render_execution_dashboard::{dashboard_group_color, render_dashboard_error};
use crate::theme::{selection_style, theme};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_execution_timeline(
        &self,
        f: &mut Frame,
        area: Rect,
        cursor: usize,
        requested_offset: usize,
    ) {
        let [header_area, table_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
        let (items, max_stages) = match self.execution_timeline_items() {
            Ok(value) => value,
            Err(error) => {
                render_dashboard_error(f, header_area, &error);
                return;
            }
        };
        let total_stages = max_stages.max(1);
        let stage_slots = usize::from(table_area.width.saturating_sub(55) / 4).max(1);
        let offset = requested_offset.min(total_stages.saturating_sub(stage_slots));
        let visible_end = (offset + stage_slots).min(total_stages);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "Dependency stages · parallel cards share a column",
                    Style::default().fg(theme().help),
                )),
                Line::from(vec![
                    Span::styled(
                        format!(" stages {}-{} of {total_stages} ", offset + 1, visible_end),
                        Style::default()
                            .fg(theme().accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" h/l scroll stages ", Style::default().fg(theme().muted)),
                ]),
            ]),
            header_area,
        );

        if items.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "No active cards to place on the execution timeline.",
                    Style::default().fg(theme().hint),
                )),
                table_area,
            );
            return;
        }

        let rows = items.iter().map(|entry| {
            let mut cells = vec![
                Cell::from(Span::styled(
                    entry.item.board.slug.clone(),
                    Style::default().fg(theme().muted),
                )),
                Cell::from(Span::styled(
                    entry.item.card.key.clone(),
                    Style::default().fg(theme().warning),
                )),
                Cell::from(Span::styled(
                    entry.item.state_label(),
                    Style::default()
                        .fg(dashboard_group_color(entry.item.group))
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    entry.item.card.title.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            for stage in offset..visible_end {
                let marker = if entry.stage == Some(stage) {
                    if entry.item.card.due_date.is_some() {
                        "█◆"
                    } else {
                        "██"
                    }
                } else if entry.stage.is_none() && stage == offset {
                    "??"
                } else {
                    "  "
                };
                cells.push(Cell::from(Span::styled(
                    marker,
                    Style::default()
                        .fg(dashboard_group_color(entry.item.group))
                        .add_modifier(Modifier::BOLD),
                )));
            }
            Row::new(cells)
        });

        let mut header_cells = vec![
            "BOARD".to_string(),
            "CARD".to_string(),
            "STATE".to_string(),
            "TITLE".to_string(),
        ];
        let mut constraints = vec![
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Min(18),
        ];
        for stage in offset..visible_end {
            header_cells.push(format!("S{}", stage + 1));
            constraints.push(Constraint::Length(3));
        }
        let header = Row::new(header_cells)
            .style(
                Style::default()
                    .fg(theme().hint)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);
        let table = Table::new(rows, constraints)
            .header(header)
            .column_spacing(1)
            .row_highlight_style(selection_style())
            .highlight_symbol(theme().selection_symbol);
        let mut state = TableState::default();
        state.select(Some(cursor.min(items.len() - 1)));
        f.render_stateful_widget(table, table_area, &mut state);
    }
}
