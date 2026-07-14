use super::App;
use crate::app::execution_dashboard::{DashboardCounts, DashboardGroup};
use crate::mode::ExecutionDashboardView;
use crate::theme::{selection_style, theme};
use kanterm_core::now_ms;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_execution_dashboard(
        &self,
        f: &mut Frame,
        area: Rect,
        view: ExecutionDashboardView,
        cursor: usize,
        focus: usize,
    ) {
        let block = dashboard_block(view);
        let inner = block.inner(area);
        f.render_widget(block, area);
        match view {
            ExecutionDashboardView::List => self.draw_execution_list(f, inner, cursor),
            ExecutionDashboardView::Timeline => {
                self.draw_execution_timeline(f, inner, cursor, focus)
            }
        }
    }

    fn draw_execution_list(&self, f: &mut Frame, area: Rect, cursor: usize) {
        let [summary_area, table_area] =
            Layout::vertical([Constraint::Length(4), Constraint::Min(1)]).areas(area);
        let items = match self.execution_dashboard_items() {
            Ok(items) => items,
            Err(error) => {
                render_dashboard_error(f, summary_area, &error);
                return;
            }
        };
        let counts = DashboardCounts::from_items(&items);
        let count_line = |groups: &[DashboardGroup]| {
            let mut spans = Vec::new();
            for group in groups {
                spans.push(Span::styled(
                    format!(" {} {} ", group.label(), counts.get(*group)),
                    Style::default()
                        .fg(dashboard_group_color(*group))
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            }
            spans
        };
        let primary = count_line(&DashboardGroup::ALL[..3]);
        let mut secondary = vec![Span::styled(
            " active board  ".to_string(),
            Style::default().fg(theme().muted),
        )];
        secondary.extend(count_line(&DashboardGroup::ALL[3..]));
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    format!("{} · live execution state", self.board.name),
                    Style::default().fg(theme().help),
                )),
                Line::from(primary),
                Line::from(secondary),
            ]),
            summary_area,
        );

        if items.is_empty() {
            render_dashboard_empty(f, table_area);
            return;
        }

        let now = now_ms();
        let rows = items.iter().map(|item| execution_row(item, now));
        let mut state = TableState::default();
        state.select(Some(cursor.min(items.len() - 1)));
        f.render_stateful_widget(execution_table(rows), table_area, &mut state);
    }
}

fn dashboard_block(view: ExecutionDashboardView) -> Block<'static> {
    let mut title = vec![Span::styled(
        " execution dashboard  ",
        Style::default()
            .fg(theme().accent)
            .add_modifier(Modifier::BOLD),
    )];
    title.push(Span::styled(" KANBAN ", Style::default().fg(theme().muted)));
    title.push(Span::raw(" "));
    for candidate in [
        ExecutionDashboardView::List,
        ExecutionDashboardView::Timeline,
    ] {
        let style = if candidate == view {
            selection_style()
        } else {
            Style::default().fg(theme().muted)
        };
        title.push(Span::styled(format!(" {} ", candidate.label()), style));
        title.push(Span::raw(" "));
    }
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme().accent))
        .title(Line::from(title))
}

pub(crate) fn dashboard_group_color(group: DashboardGroup) -> ratatui::style::Color {
    match group {
        DashboardGroup::Running => theme().accent,
        DashboardGroup::Human => theme().warning,
        DashboardGroup::Ready => theme().success,
        DashboardGroup::Blocked => theme().danger,
        DashboardGroup::Waiting => theme().priority_low,
        DashboardGroup::Missing => theme().muted,
    }
}

pub(crate) fn execution_row(
    item: &crate::app::execution_dashboard::DashboardItem,
    now: i64,
) -> Row<'static> {
    Row::new(vec![
        Cell::from(Span::styled(
            item.state_label(),
            Style::default()
                .fg(dashboard_group_color(item.group))
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            item.card.key.clone(),
            Style::default().fg(theme().warning),
        )),
        Cell::from(Span::styled(
            item.board.slug.clone(),
            Style::default().fg(theme().muted),
        )),
        Cell::from(Span::styled(
            item.card.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            item.signal(now),
            Style::default().fg(theme().help),
        )),
    ])
}

pub(crate) fn execution_table<'a>(rows: impl IntoIterator<Item = Row<'a>>) -> Table<'a> {
    let header = Row::new(["STATE", "CARD", "BOARD", "TITLE", "WHY / NEXT"])
        .style(
            Style::default()
                .fg(theme().hint)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);
    Table::new(
        rows,
        [
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Min(16),
            Constraint::Length(22),
        ],
    )
    .header(header)
    .column_spacing(1)
    .row_highlight_style(selection_style())
    .highlight_symbol(theme().selection_symbol)
}

pub(crate) fn render_dashboard_error(f: &mut Frame, area: Rect, error: &anyhow::Error) {
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("dashboard error: ", Style::default().fg(theme().danger)),
            Span::raw(error.to_string()),
        ])),
        area,
    );
}

fn render_dashboard_empty(f: &mut Frame, area: Rect) {
    f.render_widget(
        Paragraph::new(Span::styled(
            "No active execution work. Add next_action and acceptance_criteria to make a card ready.",
            Style::default().fg(theme().hint),
        )),
        area,
    );
}
