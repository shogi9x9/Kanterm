use super::App;
use crate::mode::Mode;
use crate::theme::theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

impl App {
    pub(crate) fn draw(&self, f: &mut Frame) {
        let [header_area, board_area, status_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(f.area());

        if let Mode::ExecutionDashboard {
            view,
            cursor,
            focus,
        } = &self.mode
        {
            let dashboard_area = Rect::new(
                f.area().x,
                f.area().y,
                f.area().width,
                f.area().height.saturating_sub(status_area.height),
            );
            self.draw_execution_dashboard(f, dashboard_area, *view, *cursor, *focus);
            self.draw_status(f, status_area);
            return;
        }

        self.draw_header(f, header_area);

        if self.columns.is_empty() {
            f.render_widget(Paragraph::new("no columns"), board_area);
        } else {
            let constraints: Vec<Constraint> = self
                .columns
                .iter()
                .map(|_| Constraint::Ratio(1, self.columns.len() as u32))
                .collect();
            let cols = Layout::horizontal(constraints)
                .spacing(theme().column_spacing)
                .split(board_area);
            for (i, area) in cols.iter().enumerate() {
                self.draw_column(f, i, *area);
            }
        }

        self.draw_status(f, status_area);

        match &self.mode {
            Mode::Detail { key, scroll } => self.draw_detail(f, key, *scroll),
            Mode::AgentMetadata { key, scroll } => self.draw_agent_metadata(f, key, *scroll),
            Mode::DependencyGraph { scroll } => self.draw_dependency_graph(f, *scroll),
            Mode::ExecutionDashboard { .. } => {}
            Mode::BodyEdit { key, editor, .. } => self.draw_body_edit(f, key, editor),
            Mode::Input { kind, buffer } => self.draw_input_popup(f, kind.label(), buffer),
            Mode::LabelPicker {
                key,
                input,
                cursor,
                candidates,
            } => self.draw_label_picker(f, key, input, *cursor, candidates),
            Mode::ColumnManager => self.draw_column_manager(f),
            Mode::ColumnDelete { victim_id, cursor } => {
                self.draw_column_delete(f, victim_id, *cursor)
            }
            Mode::MemoryBrowser { cursor } => self.draw_memory_browser(f, *cursor),
            Mode::MemoryDetail { key, .. } => self.draw_memory_detail(f, key),
            Mode::MemoryArchive { key, .. } => self.draw_memory_archive(f, key),
            Mode::BoardArchive {
                board_id: _,
                board_name,
            } => self.draw_board_archive(f, board_name),
            Mode::BoardSwitcher { cursor } => self.draw_board_switcher(f, *cursor),
            Mode::BoardTemplatePicker { name, cursor } => {
                self.draw_board_template_picker(f, name, *cursor)
            }
            Mode::CardBoardMove { key, cursor, .. } => self.draw_card_board_move(f, key, *cursor),
            Mode::CardColumnMove {
                key,
                board_id,
                board_name,
                cursor,
                ..
            } => self.draw_card_column_move(f, key, board_id, board_name, *cursor),
            Mode::BoardUnarchive { cursor } => self.draw_board_unarchive(f, *cursor),
            Mode::BoardDelete {
                board_id: _,
                board_name,
                confirm,
            } => self.draw_board_delete(f, board_name, confirm),
            Mode::ArchiveConfirm { key, .. } => self.draw_archive_confirm(f, key),
            Mode::Normal => {}
        }
    }
}
