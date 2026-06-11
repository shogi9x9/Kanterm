use super::App;
use crate::layout::centered_box;
use crate::theme::{hex_to_color, theme};
use kanban_core::Label;
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph,
};
use ratatui::Frame;

impl App {
    pub(crate) fn draw_input_popup(&self, f: &mut Frame, label: &str, buffer: &str) {
        let label_len = label.chars().count() as u16;
        let input_len = buffer.chars().count() as u16;
        let width = (56u16)
            .max(label_len.saturating_add(16))
            .max(input_len.saturating_add(12))
            .min(f.area().width.saturating_sub(4));
        let area = centered_box(f.area(), width, 6);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(format!(" {label} "))
            .padding(Padding::new(1, 1, 1, 1));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [input_area, hint_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(theme().success)),
                Span::raw(buffer.to_string()),
            ])),
            input_area,
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                "Enter で確定  / Esc で中断",
                Style::default().fg(theme().hint),
            )),
            hint_area,
        );
        let max_cursor = input_area.width.saturating_sub(3) as usize;
        let cursor = buffer.chars().count().min(max_cursor) as u16;
        f.set_cursor_position(Position::new(input_area.x + 2 + cursor, input_area.y));
    }

    pub(crate) fn draw_label_picker(
        &self,
        f: &mut Frame,
        key: &str,
        input: &str,
        cursor: usize,
        candidates: &[Label],
    ) {
        let area = centered_box(f.area(), 48, (candidates.len() as u16 + 5).clamp(7, 20));
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(format!(" labels: {key} "));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let [input_area, list_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("+ ", Style::default().fg(theme().success)),
                Span::raw(input.to_string()),
            ])),
            input_area,
        );

        let items: Vec<ListItem> = candidates
            .iter()
            .map(|l| {
                let on = self.card_has_label(key, &l.name);
                let check = if on { "[x] " } else { "[ ] " };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        check,
                        Style::default().fg(if on { theme().success } else { theme().help }),
                    ),
                    Span::styled(
                        format!("●{}", l.name),
                        Style::default().fg(hex_to_color(&l.color)),
                    ),
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
        if !candidates.is_empty() {
            state.select(Some(cursor.min(candidates.len() - 1)));
        }
        f.render_stateful_widget(list, list_area, &mut state);
        f.set_cursor_position(Position::new(
            input_area.x + 2 + input.chars().count() as u16,
            input_area.y,
        ));
    }

    pub(crate) fn draw_column_manager(&self, f: &mut Frame) {
        let h = (self.columns.len() as u16 + 4).clamp(6, 20);
        let area = centered_box(f.area(), 46, h);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().accent))
            .title(format!(" columns · {} ", self.board.name));
        let items: Vec<ListItem> = self
            .columns
            .iter()
            .map(|c| {
                let count = self
                    .cards
                    .iter()
                    .filter(|card| card.column_id == c.id)
                    .count();
                ListItem::new(Line::from(format!("{}  ({} cards)", c.name, count)))
            })
            .collect();
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme().selected_bg)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default();
        if !self.columns.is_empty() {
            state.select(Some(self.col_cursor.min(self.columns.len() - 1)));
        }
        f.render_stateful_widget(list, area, &mut state);
    }

    pub(crate) fn draw_column_delete(&self, f: &mut Frame, victim_id: &str, cursor: usize) {
        let dests = self.delete_destinations(victim_id);
        let victim = self
            .columns
            .iter()
            .find(|c| c.id == victim_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let h = (dests.len() as u16 + 4).clamp(6, 20);
        let area = centered_box(f.area(), 50, h);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme().warning))
            .title(format!(" delete '{victim}' → move cards to "));
        let items: Vec<ListItem> = dests
            .iter()
            .map(|(_, name)| ListItem::new(Line::from(name.clone())))
            .collect();
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme().warning)
                .fg(theme().selected_fg)
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default();
        if !dests.is_empty() {
            state.select(Some(cursor.min(dests.len() - 1)));
        }
        f.render_stateful_widget(list, area, &mut state);
    }
}
