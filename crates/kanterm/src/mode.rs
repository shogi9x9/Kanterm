use crate::editor::Editor;
use kanterm_core::Label;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionDashboardView {
    List,
    Timeline,
    Flow,
}

impl ExecutionDashboardView {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::List => Self::Timeline,
            Self::Timeline => Self::Flow,
            Self::Flow => Self::List,
        }
    }

    pub(crate) fn previous(self) -> Self {
        match self {
            Self::List => Self::Flow,
            Self::Timeline => Self::List,
            Self::Flow => Self::Timeline,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::List => "LIST",
            Self::Timeline => "TIMELINE",
            Self::Flow => "FLOW",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionDashboardState {
    pub(crate) view: ExecutionDashboardView,
    pub(crate) cursor: usize,
    pub(crate) focus: usize,
}

impl ExecutionDashboardState {
    pub(crate) const fn new(view: ExecutionDashboardView, cursor: usize, focus: usize) -> Self {
        Self {
            view,
            cursor,
            focus,
        }
    }
}

pub(crate) enum Mode {
    Normal,
    Detail {
        key: String,
        scroll: u16,
    },
    AgentMetadata {
        key: String,
        scroll: u16,
    },
    DependencyGraph {
        scroll: u16,
    },
    /// Board-scoped views of executable, running, human-gated and blocked work.
    ExecutionDashboard(ExecutionDashboardState),
    Input {
        kind: InputKind,
        buffer: String,
    },
    BodyEdit {
        key: String,
        editor: Editor,
        expected_updated_at: i64,
    },
    /// Multi-label picker for one card: a text field to add a new label plus a
    /// navigable list of recently used labels to toggle on/off.
    LabelPicker {
        key: String,
        input: String,
        cursor: usize,
        candidates: Vec<Label>,
    },
    /// Column (status) management for the current board.
    ColumnManager,
    /// Pick the destination column for the cards of a column being deleted.
    ColumnDelete {
        victim_id: String,
        cursor: usize,
    },
    /// y/n confirmation for archiving the current board (reversible).
    BoardArchive {
        board_id: String,
        board_name: String,
    },
    /// Picker over active boards.
    BoardSwitcher {
        cursor: usize,
    },
    /// Picker over built-in column templates before creating a new board.
    BoardTemplatePicker {
        name: String,
        cursor: usize,
    },
    /// Picker over active destination boards for moving one card.
    CardBoardMove {
        key: String,
        cursor: usize,
        back: ArchiveBack,
    },
    /// Picker over destination columns after a destination board was chosen.
    CardColumnMove {
        key: String,
        board_id: String,
        board_name: String,
        cursor: usize,
        back: ArchiveBack,
    },
    /// Picker over archived boards: Enter unarchives, d hard-deletes.
    BoardUnarchive {
        cursor: usize,
    },
    /// Memory log browser: newest-first list of recorded decisions/learnings.
    MemoryBrowser {
        cursor: usize,
    },
    /// One memory in full (title + body + meta).
    MemoryDetail {
        key: String,
        cursor: usize,
    },
    /// y/n confirmation for archiving a memory from the browser.
    MemoryArchive {
        key: String,
        cursor: usize,
    },
    /// Typed confirmation for permanently deleting an (archived) board.
    BoardDelete {
        board_id: String,
        board_name: String,
        confirm: String,
    },
    ArchiveConfirm {
        key: String,
        back: ArchiveBack,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum ArchiveBack {
    Normal,
    Detail,
}

pub(crate) enum InputKind {
    NewCard,
    EditTitle {
        key: String,
        expected_updated_at: i64,
    },
    EditAssignee {
        key: String,
        expected_updated_at: i64,
    },
    EditDue {
        key: String,
        expected_updated_at: i64,
    },
    CompleteWithNote {
        key: String,
        expected_updated_at: i64,
    },
    Filter,
    NewBoard,
    EditBoardContext,
    NewColumn,
    RenameColumn(String),
}

impl InputKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            InputKind::NewCard => "new card",
            InputKind::EditTitle { .. } => "edit title",
            InputKind::EditAssignee { .. } => "assignee",
            InputKind::EditDue { .. } => "due (YYYY-MM-DD)",
            InputKind::CompleteWithNote { .. } => "complete note (optional)",
            InputKind::Filter => "filter",
            InputKind::NewBoard => "new board name",
            InputKind::EditBoardContext => "board agent context (empty clears)",
            InputKind::NewColumn => "new column name",
            InputKind::RenameColumn(_) => "rename column",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExecutionDashboardView;

    #[test]
    fn dashboard_views_cycle_in_both_directions() {
        assert_eq!(
            ExecutionDashboardView::List.next(),
            ExecutionDashboardView::Timeline
        );
        assert_eq!(
            ExecutionDashboardView::Timeline.next(),
            ExecutionDashboardView::Flow
        );
        assert_eq!(
            ExecutionDashboardView::Flow.next(),
            ExecutionDashboardView::List
        );
        assert_eq!(
            ExecutionDashboardView::List.previous(),
            ExecutionDashboardView::Flow
        );
    }
}
