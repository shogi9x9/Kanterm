#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardColumnTemplate {
    Planning,
    Workflow,
    Simple,
}

impl BoardColumnTemplate {
    pub const ALL: &[BoardColumnTemplate] = &[
        BoardColumnTemplate::Planning,
        BoardColumnTemplate::Workflow,
        BoardColumnTemplate::Simple,
    ];

    pub const DEFAULT_PROJECT: BoardColumnTemplate = BoardColumnTemplate::Workflow;

    pub fn key(self) -> &'static str {
        match self {
            BoardColumnTemplate::Planning => "planning",
            BoardColumnTemplate::Workflow => "workflow",
            BoardColumnTemplate::Simple => "simple",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BoardColumnTemplate::Planning => "Planning",
            BoardColumnTemplate::Workflow => "Workflow",
            BoardColumnTemplate::Simple => "Simple",
        }
    }

    pub fn columns(self) -> &'static [&'static str] {
        match self {
            BoardColumnTemplate::Planning => &["Backlog", "Today", "This week", "This month"],
            BoardColumnTemplate::Workflow => {
                &["Todo", "In progress", "Testing", "Waiting for release"]
            }
            BoardColumnTemplate::Simple => &["Todo", "Doing", "Done"],
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.key() == key)
    }

    pub fn default_index() -> usize {
        Self::ALL
            .iter()
            .position(|t| *t == Self::DEFAULT_PROJECT)
            .unwrap_or(0)
    }
}
