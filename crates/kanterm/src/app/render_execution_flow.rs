use super::App;
use crate::app::execution_dashboard::{DashboardCounts, DashboardGroup, FLOW_GROUPS};
use crate::app::render_execution_dashboard::{
    dashboard_group_color, execution_row, execution_table, render_dashboard_error,
};
use crate::theme::{selection_style, theme};
use kanterm_core::now_ms;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, TableState};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_execution_flow(
        &self,
        f: &mut Frame,
        area: Rect,
        cursor: usize,
        focus: usize,
    ) {
        let [graph_area, selected_area, table_area] = Layout::vertical([
            Constraint::Length(7),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .areas(area);
        let all_items = match self.execution_dashboard_items() {
            Ok(items) => items,
            Err(error) => {
                render_dashboard_error(f, graph_area, &error);
                return;
            }
        };
        let counts = DashboardCounts::from_items(&all_items);
        let selected_group = FLOW_GROUPS[focus.min(FLOW_GROUPS.len() - 1)];
        let node = |group| flow_node(group, counts.get(group), group == selected_group);

        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "Derived states · every change is re-classified by kanterm-core",
                    Style::default().fg(theme().help),
                )),
                Line::from(vec![
                    node(DashboardGroup::Missing),
                    Span::styled(" ─context──┐", Style::default().fg(theme().muted)),
                ]),
                Line::from(vec![
                    node(DashboardGroup::Waiting),
                    Span::styled(" ─deps─────┤", Style::default().fg(theme().muted)),
                ]),
                Line::from(vec![
                    node(DashboardGroup::Blocked),
                    Span::styled(" ─clear───├▶ ", Style::default().fg(theme().muted)),
                    node(DashboardGroup::Ready),
                    Span::styled(" ─claim▶ ", Style::default().fg(theme().muted)),
                    node(DashboardGroup::Running),
                    Span::styled(" ─done▶ DONE", Style::default().fg(theme().muted)),
                ]),
                Line::from(vec![
                    node(DashboardGroup::Human),
                    Span::styled(" ─approve───┘", Style::default().fg(theme().muted)),
                ]),
                Line::from(Span::styled(
                    "RUNNING ─release / lease expiry─▶ re-evaluate",
                    Style::default().fg(theme().muted),
                )),
            ]),
            graph_area,
        );

        let selected_items = all_items
            .iter()
            .filter(|item| item.group == selected_group)
            .collect::<Vec<_>>();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" selected state  ", Style::default().fg(theme().muted)),
                flow_node(selected_group, selected_items.len(), true),
                Span::styled(
                    "   h/l state · j/k cards",
                    Style::default().fg(theme().help),
                ),
            ])),
            selected_area,
        );
        if selected_items.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "No cards currently derive to this state.",
                    Style::default().fg(theme().hint),
                )),
                table_area,
            );
            return;
        }

        let now = now_ms();
        let rows = selected_items.iter().map(|item| execution_row(item, now));
        let mut state = TableState::default();
        state.select(Some(cursor.min(selected_items.len() - 1)));
        f.render_stateful_widget(execution_table(rows), table_area, &mut state);
    }
}

fn flow_node(group: DashboardGroup, count: usize, selected: bool) -> Span<'static> {
    let style = if selected {
        selection_style()
    } else {
        Style::default()
            .fg(dashboard_group_color(group))
            .add_modifier(Modifier::BOLD)
    };
    Span::styled(format!("[ {} {count} ]", group.label()), style)
}
