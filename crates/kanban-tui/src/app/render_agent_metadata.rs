use super::{claim_is_active, App};
use crate::layout::centered;
use crate::theme::theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

impl App {
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
}

fn optional_i64(value: Option<i64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
