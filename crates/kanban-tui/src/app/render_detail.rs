use super::{claim_is_active, App};
use crate::editor::Editor;
use crate::layout::centered;
use crate::theme::{priority_span, theme};
use kanban_core::{format_date, today_start_ms};
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

    pub(crate) fn draw_agent_metadata(&self, f: &mut Frame, key: &str, scroll: u16) {
        let Some(c) = self.card_by_key(key) else {
            return;
        };
        let area = centered(f.area(), 64, 56);
        f.render_widget(Clear, area);

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", c.key),
                    Style::default()
                        .fg(theme().warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "agent metadata",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("agent_state: ", Style::default().fg(theme().accent)),
                Span::raw(c.agent_state.clone()),
            ]),
            Line::from(vec![
                Span::styled("claim: ", Style::default().fg(theme().accent)),
                Span::raw(claim_value(
                    c.claimed_by.as_deref(),
                    c.lease_expires_at,
                    claim_is_active(c),
                )),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("agent_weight: ", Style::default().fg(theme().accent)),
                Span::raw(optional_i64(c.agent_weight)),
            ]),
            Line::from(vec![
                Span::styled("agent_effort: ", Style::default().fg(theme().accent)),
                Span::raw(c.agent_effort.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(vec![
                Span::styled("suggested_model: ", Style::default().fg(theme().accent)),
                Span::raw(c.suggested_model.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(vec![
                Span::styled("expected_tokens: ", Style::default().fg(theme().accent)),
                Span::raw(optional_i64(c.expected_tokens)),
            ]),
            Line::from(vec![
                Span::styled("human_intervention: ", Style::default().fg(theme().warning)),
                Span::raw(c.human_intervention.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("next_action: ", Style::default().fg(theme().accent)),
                Span::raw(c.next_action.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(vec![
                Span::styled("blocked_reason: ", Style::default().fg(theme().danger)),
                Span::raw(c.blocked_reason.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(vec![
                Span::styled(
                    "acceptance_criteria: ",
                    Style::default().fg(theme().success),
                ),
                Span::raw(c.acceptance_criteria.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(vec![
                Span::styled("handoff_note: ", Style::default().fg(theme().warning)),
                Span::raw(c.handoff_note.as_deref().unwrap_or("-").to_string()),
            ]),
            Line::from(vec![
                Span::styled("last_verification: ", Style::default().fg(theme().success)),
                Span::raw(c.last_verification.as_deref().unwrap_or("-").to_string()),
            ]),
        ];
        lines.push(Line::from(vec![
            Span::styled("created_at: ", Style::default().fg(theme().muted)),
            Span::raw(c.created_at.to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("updated_at: ", Style::default().fg(theme().muted)),
            Span::raw(c.updated_at.to_string()),
        ]));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(" agent metadata ");
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

fn optional_i64(value: Option<i64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
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

fn claim_value(claimed_by: Option<&str>, lease_expires_at: Option<i64>, active: bool) -> String {
    match (claimed_by, lease_expires_at) {
        (Some(claimed_by), Some(expires_at)) if active => {
            format!("{claimed_by} until lease_expires_at={expires_at}")
        }
        (Some(claimed_by), Some(expires_at)) => {
            format!("expired {claimed_by} at lease_expires_at={expires_at}")
        }
        (Some(claimed_by), None) => format!("{claimed_by} without lease"),
        _ => "-".into(),
    }
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
    use kanban_core::parse_date;

    #[test]
    fn claim_value_marks_active_and_expired_claims() {
        assert_eq!(
            claim_value(Some("agent"), Some(123), true),
            "agent until lease_expires_at=123"
        );
        assert_eq!(
            claim_value(Some("agent"), Some(123), false),
            "expired agent at lease_expires_at=123"
        );
        assert_eq!(
            claim_value(Some("agent"), None, false),
            "agent without lease"
        );
        assert_eq!(claim_value(None, None, false), "-");
    }

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
