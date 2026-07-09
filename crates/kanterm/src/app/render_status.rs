use super::App;
use crate::mode::Mode;
use crate::theme::theme;
use kanterm_core::{priority_badge, PROTECTED_BOARD_SLUG};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

impl App {
    pub(crate) fn draw_status(&self, f: &mut Frame, area: Rect) {
        let line = match &self.mode {
            Mode::Input { .. } => Line::from(Span::styled(
                " Enter confirm   Esc cancel (editing) ",
                Style::default().fg(theme().help),
            )),
            Mode::LabelPicker { .. } => Line::from(Span::styled(
                " ↑/↓ move   Space: toggle   type + Enter: add new   Esc: done ",
                Style::default().fg(theme().selected_fg).bg(theme().success),
            )),
            Mode::ColumnManager => Line::from(Span::styled(
                " COLUMNS  j/k move  a add  r rename  d delete  J/K reorder  Esc back ",
                Style::default().fg(theme().hint),
            )),
            Mode::ColumnDelete { .. } => Line::from(Span::styled(
                " pick where this column's cards should go - ↑/↓ + Enter, Esc cancel ",
                Style::default().fg(theme().selected_fg).bg(theme().warning),
            )),
            Mode::MemoryBrowser { .. } => Line::from(Span::styled(
                " MEMORIES  j/k move  Enter detail  d archive  Esc back ",
                Style::default().fg(theme().hint),
            )),
            Mode::MemoryDetail { .. } => Line::from(Span::styled(
                " MEMORY  Esc back ",
                Style::default().fg(theme().hint),
            )),
            Mode::MemoryArchive { .. } => Line::from(Span::styled(
                " Archive memory?  y archive  n/Esc cancel ",
                Style::default().fg(theme().selected_fg).bg(theme().warning),
            )),
            Mode::BoardArchive { .. } => Line::from(Span::styled(
                " Archive board?  y archive  n/Esc cancel ",
                Style::default().fg(theme().selected_fg).bg(theme().warning),
            )),
            Mode::BoardSwitcher { .. } => Line::from(Span::styled(
                " BOARDS  j/k select  J/K reorder  Enter switch  Esc back ",
                Style::default().fg(theme().hint),
            )),
            Mode::BoardTemplatePicker { .. } => Line::from(Span::styled(
                " BOARD TEMPLATE  j/k select  Enter create  Esc cancel ",
                Style::default().fg(theme().hint),
            )),
            Mode::CardBoardMove { .. } => {
                let help = if self.board.slug == PROTECTED_BOARD_SLUG {
                    " SEND TO PROJECT  j/k select board  Enter choose column  M/Esc cancel "
                } else {
                    " MOVE CARD  j/k select board  Enter choose column  M/Esc cancel "
                };
                Line::from(Span::styled(help, Style::default().fg(theme().hint)))
            }
            Mode::CardColumnMove { .. } => {
                let help = if self.board.slug == PROTECTED_BOARD_SLUG {
                    " SEND TO PROJECT  j/k select column  Enter send  b boards  M/Esc cancel "
                } else {
                    " MOVE CARD  j/k select column  Enter move  b boards  M/Esc cancel "
                };
                Line::from(Span::styled(help, Style::default().fg(theme().hint)))
            }
            Mode::BoardUnarchive { .. } => Line::from(Span::styled(
                " ARCHIVED BOARDS  j/k move  Enter unarchive  d delete forever  Esc back ",
                Style::default().fg(theme().selected_fg).bg(theme().warning),
            )),
            Mode::BoardDelete { .. } => Line::from(Span::styled(
                " Board delete (permanent)  type delete + Enter to confirm  Esc to cancel ",
                Style::default().fg(theme().selected_fg).bg(theme().danger),
            )),
            Mode::ArchiveConfirm { .. } => Line::from(Span::styled(
                " Archive card?  y archive  n/Esc cancel ",
                Style::default().fg(theme().selected_fg).bg(theme().warning),
            )),
            Mode::BodyEdit { .. } => Line::from(Span::styled(
                " BODY  arrows move  Enter newline  Ctrl-S save  Esc cancel ",
                Style::default().fg(theme().selected_fg).bg(theme().success),
            )),
            Mode::Detail { .. } => {
                let help = if self.board.slug == PROTECTED_BOARD_SLUG {
                    " DETAIL  e title  b body  M send-project  p priority  a assignee  D due  t labels  m metadata  x complete+note  d archive  Esc back "
                } else {
                    " DETAIL  e title  b body  M move-board  p priority  a assignee  D due  t labels  m metadata  x complete+note  d archive  Esc back "
                };
                Line::from(Span::styled(help, Style::default().fg(theme().hint)))
            }
            Mode::AgentMetadata { .. } => Line::from(Span::styled(
                " AGENT METADATA  j/k scroll  m/Esc back ",
                Style::default().fg(theme().hint),
            )),
            Mode::DependencyGraph { .. } => Line::from(Span::styled(
                " DEPENDENCY GRAPH  j/k scroll  g/Esc back ",
                Style::default().fg(theme().hint),
            )),
            Mode::Normal => {
                let detail = self
                    .selected_card()
                    .map(|c| format!("{} {}", c.key, priority_badge(c.priority)))
                    .unwrap_or_else(|| "-".into());
                let help = if self.board.slug == PROTECTED_BOARD_SLUG {
                    " j/k nav  J/K move  n new  b boards  i context  M send-project  w work  g graph  v human  m memories  / filter  ↵ detail  p prio  d arch  q quit "
                } else {
                    " h/l j/k nav  H/L/J/K move  n new  b boards  i context  M move-board  c columns  w work  g graph  v human  m memories  / filter  ↵ detail  p prio  d arch  q quit "
                };
                let mut spans = vec![Span::styled(help, Style::default().fg(theme().help))];
                if self.board.agent_context.is_some() {
                    spans.push(Span::styled(
                        "[board-context] ",
                        Style::default().fg(theme().selected_fg).bg(theme().success),
                    ));
                }
                if let Some(f) = &self.filter {
                    spans.push(Span::styled(
                        format!("[filter: {f}] "),
                        Style::default().fg(theme().selected_fg).bg(theme().warning),
                    ));
                }
                spans.push(Span::styled(detail, Style::default().fg(theme().success)));
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    self.status.clone(),
                    Style::default().fg(theme().accent),
                ));
                Line::from(spans)
            }
        };
        f.render_widget(Paragraph::new(line), area);
    }
}
