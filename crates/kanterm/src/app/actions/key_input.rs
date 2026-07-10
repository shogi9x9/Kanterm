use anyhow::Result;
use crossterm::event::KeyEvent;

use crate::app::App;
use crate::mode::Mode;

impl App {
    pub(crate) fn on_key(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.mode {
            Mode::Normal => self.on_normal_key(key)?,
            Mode::Detail { .. } => self.on_detail_key(key)?,
            Mode::AgentMetadata { .. } => self.on_agent_metadata_key(key)?,
            Mode::DependencyGraph { .. } => self.on_dependency_graph_key(key),
            Mode::ExecutionDashboard(_) => self.on_execution_dashboard_key(key)?,
            Mode::Input { .. } => self.on_input_key(key)?,
            Mode::BodyEdit { .. } => self.on_body_key(key)?,
            Mode::LabelPicker { .. } => self.on_label_key(key)?,
            Mode::ColumnManager => self.on_columns_key(key)?,
            Mode::ColumnDelete { .. } => self.on_column_delete_key(key)?,
            Mode::BoardArchive { .. } => self.on_board_archive_key(key)?,
            Mode::BoardSwitcher { .. } => self.on_board_switcher_key(key)?,
            Mode::BoardTemplatePicker { .. } => self.on_board_template_key(key)?,
            Mode::CardBoardMove { .. } => self.on_card_board_move_key(key)?,
            Mode::CardColumnMove { .. } => self.on_card_column_move_key(key)?,
            Mode::BoardUnarchive { .. } => self.on_board_unarchive_key(key)?,
            Mode::BoardDelete { .. } => self.on_board_delete_key(key)?,
            Mode::MemoryBrowser { .. } => self.on_memory_browser_key(key)?,
            Mode::MemoryDetail { .. } => self.on_memory_detail_key(key)?,
            Mode::MemoryArchive { .. } => self.on_memory_archive_key(key)?,
            Mode::ArchiveConfirm { .. } => self.on_archive_confirm_key(key)?,
        }
        Ok(())
    }
}
