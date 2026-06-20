use super::App;
use crate::layout::centered;
use crate::theme::theme;
use kanban_core::{classify_graph_node, now_ms, DependencyStagePlan};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::HashMap;

impl App {
    pub(crate) fn draw_dependency_graph(&self, f: &mut Frame, scroll: u16) {
        let area = centered(f.area(), 78, 70);
        f.render_widget(Clear, area);

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    self.board.name.clone(),
                    Style::default()
                        .fg(theme().warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" dependency graph"),
            ]),
            Line::from(""),
        ];

        match self.dependency_graph_lines() {
            Ok(mut graph_lines) => lines.append(&mut graph_lines),
            Err(err) => lines.push(Line::from(vec![
                Span::styled("error: ", Style::default().fg(theme().danger)),
                Span::raw(err.to_string()),
            ])),
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(" dependency graph ");
        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            area,
        );
    }

    fn dependency_graph_lines(&self) -> anyhow::Result<Vec<Line<'static>>> {
        let dependencies = self.store.card_dependencies(&self.board.id)?;
        let stages = self.store.dependency_stage_plan(&self.board.id)?;
        let states = self.node_states(&stages)?;
        let mut lines = vec![Line::from(Span::styled(
            "stages",
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        ))];
        if stages.ready_stages.is_empty() {
            lines.push(Line::from("-"));
        } else {
            for (idx, stage) in stages.ready_stages.iter().enumerate() {
                let nodes = stage
                    .iter()
                    .map(|key| format_node(key, &states))
                    .collect::<Vec<_>>();
                lines.push(Line::from(format!(
                    "stage {}: {}",
                    idx + 1,
                    nodes.join("  ")
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "dependency blocked",
            Style::default()
                .fg(theme().warning)
                .add_modifier(Modifier::BOLD),
        )));
        if stages.dependency_blocked.is_empty() {
            lines.push(Line::from("-"));
        } else {
            for blocked in stages.dependency_blocked {
                lines.push(Line::from(format!(
                    "{} blocked_by: {}",
                    format_node(&blocked.key, &states),
                    blocked
                        .blocked_by
                        .iter()
                        .map(|key| format_node(key, &states))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "edges",
            Style::default()
                .fg(theme().muted)
                .add_modifier(Modifier::BOLD),
        )));
        if dependencies.is_empty() {
            lines.push(Line::from("-"));
        } else {
            for dep in dependencies {
                lines.push(Line::from(format!(
                    "{} -> {}",
                    dep.upstream_key, dep.downstream_key
                )));
            }
        }
        Ok(lines)
    }

    fn node_states(&self, stages: &DependencyStagePlan) -> anyhow::Result<HashMap<String, String>> {
        let mut states = HashMap::new();
        let now = now_ms();
        for key in stages
            .ready_stages
            .iter()
            .flatten()
            .chain(stages.dependency_blocked.iter().map(|blocked| &blocked.key))
        {
            if let Some(card) = self.card_by_key(key) {
                let readiness = self.store.card_readiness(&self.board.id, key)?;
                states.insert(
                    key.clone(),
                    classify_graph_node(card, &readiness, now).label(),
                );
            }
        }
        Ok(states)
    }
}

fn format_node(key: &str, states: &HashMap<String, String>) -> String {
    match states.get(key) {
        Some(state) => format!("{key}({state})"),
        None => key.to_string(),
    }
}
