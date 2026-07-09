use super::App;
use crate::editor::Editor;
use crate::layout::centered;
use crate::theme::{priority_span, theme};
use kanterm_core::{format_date, today_start_ms};
use ratatui::layout::Position;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_detail(&self, f: &mut Frame, key: &str, scroll: u16) {
        let Some(c) = self.card_by_key(key) else {
            return;
        };
        let area = centered(f.area(), 64, 64);
        f.render_widget(Clear, area);

        let col_name = self
            .columns
            .iter()
            .find(|col| col.id == c.column_id)
            .map(|col| col.name.clone())
            .unwrap_or_default();
        let tags = self
            .labels
            .get(&c.id)
            .map(|ls| {
                ls.iter()
                    .map(|l| l.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "-".into());
        let hint = Style::default()
            .fg(theme().hint)
            .add_modifier(Modifier::ITALIC);

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", c.key),
                    Style::default()
                        .fg(theme().warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    c.title.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw(format!("column: {col_name}   priority: ")),
                priority_span(c.priority),
            ]),
            Line::from(vec![
                Span::raw(format!(
                    "assignee: {}",
                    c.assignee.as_deref().unwrap_or("-")
                )),
                Span::styled("   (a to edit)", hint),
            ]),
            Line::from(vec![
                Span::raw(format!("labels: {tags}")),
                Span::styled("   (t to edit)", hint),
            ]),
        ];
        let (due_text, overdue) = due_text(c.due_date, today_start_ms());
        let due_span = if overdue {
            Span::styled(
                due_text,
                Style::default()
                    .fg(theme().danger)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(due_text)
        };
        lines.push(Line::from(vec![
            due_span,
            Span::styled("   (D to edit)", hint),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "resume",
            Style::default()
                .fg(theme().success)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(resume_line(
            "next_action",
            c.next_action.as_deref(),
            theme().accent,
        ));
        lines.push(resume_line(
            "acceptance",
            c.acceptance_criteria.as_deref(),
            theme().success,
        ));
        if c.blocked_reason.is_some() {
            lines.push(resume_line(
                "blocked",
                c.blocked_reason.as_deref(),
                theme().danger,
            ));
        }
        if let Some(human) = c.human_intervention.as_deref().filter(|v| *v != "none") {
            lines.push(resume_line("human", Some(human), theme().warning));
        }
        lines.push(resume_line(
            "handoff",
            c.handoff_note.as_deref(),
            theme().warning,
        ));
        lines.push(resume_line(
            "verification",
            c.last_verification.as_deref(),
            theme().success,
        ));
        lines.push(Line::from(""));
        let body = if c.body.is_empty() {
            "(no description - press b to edit)"
        } else {
            &c.body
        };
        for l in body.split('\n') {
            lines.push(Line::from(l.to_string()));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(" card ");
        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            area,
        );
    }

    pub(crate) fn draw_body_edit(&self, f: &mut Frame, key: &str, editor: &Editor) {
        let area = centered(f.area(), 70, 70);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().success))
            .title(format!(" edit body: {key} "));
        let inner = block.inner(area);
        let lines: Vec<Line> = editor.lines.iter().map(|l| Line::from(l.clone())).collect();
        f.render_widget(Paragraph::new(lines).block(block), area);
        f.set_cursor_position(Position::new(
            inner.x + editor.cursor_display_x() as u16,
            inner.y + editor.cy as u16,
        ));
    }
}

fn resume_line<'a>(
    label: &'static str,
    value: Option<&'a str>,
    color: ratatui::style::Color,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(color)),
        Span::raw(resume_value(value)),
    ])
}

fn resume_value(value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return "-".into();
    };
    value.replace('\n', " / ")
}

fn due_text(due_date: Option<i64>, today: i64) -> (String, bool) {
    match due_date {
        Some(ms) if ms < today => (format!("due: {} (overdue)", format_date(ms)), true),
        Some(ms) => (format!("due: {}", format_date(ms)), false),
        None => ("due: -".to_string(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanterm_core::parse_date;

    #[test]
    fn due_text_marks_only_past_dates_overdue() {
        let today = parse_date("2026-06-12").unwrap();
        assert_eq!(
            due_text(Some(parse_date("2026-06-11").unwrap()), today),
            ("due: 2026-06-11 (overdue)".to_string(), true)
        );
        assert_eq!(
            due_text(Some(parse_date("2026-06-12").unwrap()), today),
            ("due: 2026-06-12".to_string(), false)
        );
        assert_eq!(due_text(None, today), ("due: -".to_string(), false));
    }

    #[test]
    fn resume_value_keeps_summary_single_line() {
        assert_eq!(resume_value(None), "-");
        assert_eq!(resume_value(Some("   ")), "-");
        assert_eq!(resume_value(Some("first\nsecond")), "first / second");
    }
}
