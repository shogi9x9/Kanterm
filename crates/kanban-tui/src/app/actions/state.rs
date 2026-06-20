use anyhow::Result;

use crate::app::{claim_is_active, App, UI_FOCUS, UI_SELECTED};
use crate::mode::Mode;
use kanban_core::{Card, CardReadiness, HumanIntervention};

impl App {
    pub(crate) fn reload(&mut self) -> Result<()> {
        self.cards = self.store.cards(&self.board.id)?;
        self.labels = self.store.labels_by_card(&self.board.id)?;
        if self.cursors.len() != self.columns.len() {
            self.cursors = vec![0; self.columns.len()];
        }
        for i in 0..self.columns.len() {
            let max = self.column_cards(i).len().saturating_sub(1);
            if self.cursors[i] > max {
                self.cursors[i] = max;
            }
        }
        Ok(())
    }

    pub(crate) fn resync_external(&mut self) -> Result<()> {
        self.boards = self.store.list_boards()?;
        if let Some(board) = self.boards.iter().find(|b| b.id == self.board.id).cloned() {
            self.board = board;
            // External writers can change board/column structure, while reload()
            // intentionally only refreshes the card window for hot paths.
            self.refresh_columns()?;
        } else if let Some(board) = self.boards.first().cloned() {
            let previous = self.board.name.clone();
            self.switch_board(board)?;
            self.mode = Mode::Normal;
            self.status = format!(
                "current board '{previous}' is unavailable; switched to {}",
                self.board.name
            );
        }
        Ok(())
    }

    pub(crate) fn column_cards(&self, col: usize) -> Vec<&Card> {
        let col_id = &self.columns[col].id;
        let needle = self.filter.as_deref().map(str::to_lowercase);
        self.cards
            .iter()
            .filter(|c| &c.column_id == col_id)
            .filter(|c| match &needle {
                None => true,
                Some(q) => {
                    c.title.to_lowercase().contains(q)
                        || c.body.to_lowercase().contains(q)
                        || self
                            .labels
                            .get(&c.id)
                            .map(|ls| ls.iter().any(|l| l.name.to_lowercase().contains(q)))
                            .unwrap_or(false)
                }
            })
            .collect()
    }

    pub(crate) fn selected_card(&self) -> Option<&Card> {
        self.column_cards(self.focus)
            .get(self.cursors[self.focus])
            .copied()
    }

    pub(crate) fn card_by_key(&self, key: &str) -> Option<&Card> {
        self.cards.iter().find(|c| c.key == key)
    }

    pub(crate) fn select_key(&mut self, key: &str) {
        for i in 0..self.columns.len() {
            if let Some(pos) = self.column_cards(i).iter().position(|c| c.key == key) {
                self.focus = i;
                self.cursors[i] = pos;
                return;
            }
        }
    }

    pub(crate) fn jump_to_human_intervention(&mut self) {
        let mut candidates = Vec::new();
        for col in 0..self.columns.len() {
            for (pos, card) in self.column_cards(col).iter().enumerate() {
                if let Some(kind) = human_intervention_kind(card.human_intervention.as_deref()) {
                    candidates.push((col, pos, card.key.clone(), kind.to_string()));
                }
            }
        }

        if candidates.is_empty() {
            self.status = match &self.filter {
                Some(filter) => format!("no human-intervention cards visible for filter: {filter}"),
                None => "no human-intervention cards on this board".into(),
            };
            return;
        }

        let current = (
            self.focus,
            self.cursors.get(self.focus).copied().unwrap_or(0),
        );
        let next = candidates
            .iter()
            .position(|(col, pos, _, _)| (*col, *pos) > current)
            .unwrap_or(0);
        let (col, pos, key, kind) = &candidates[next];
        self.focus = *col;
        self.cursors[*col] = *pos;
        self.status = format!("human_intervention: {key} {kind}");
    }

    pub(crate) fn jump_to_next_work(&mut self) {
        let mut candidates = Vec::new();
        for col in 0..self.columns.len() {
            for (pos, card) in self.column_cards(col).iter().enumerate() {
                let Ok(readiness) = self.store.card_readiness(&self.board.id, &card.key) else {
                    continue;
                };
                if let Some(kind) = next_work_kind(card, &readiness) {
                    candidates.push((kind.rank(), col, pos, card.key.clone(), kind.label()));
                }
            }
        }

        if candidates.is_empty() {
            self.status = match &self.filter {
                Some(filter) => format!("no next-work cards visible for filter: {filter}"),
                None => "no next-work cards on this board".into(),
            };
            return;
        }

        candidates.sort_by_key(|(rank, col, pos, _, _)| (*rank, *col, *pos));
        let current = (
            self.focus,
            self.cursors.get(self.focus).copied().unwrap_or(0),
        );
        let next = candidates
            .iter()
            .position(|(_, col, pos, _, _)| (*col, *pos) > current)
            .unwrap_or(0);
        let (_, col, pos, key, label) = &candidates[next];
        self.focus = *col;
        self.cursors[*col] = *pos;
        self.status = format!("next-work: {key} {label}");
    }

    pub(crate) fn restore_ui_state(&mut self) {
        if let Ok(Some(v)) = self.store.get_ui_state(UI_FOCUS) {
            if let Ok(i) = v.parse::<usize>() {
                if i < self.columns.len() {
                    self.focus = i;
                }
            }
        }
        if let Ok(Some(key)) = self.store.get_ui_state(UI_SELECTED) {
            self.select_key(&key);
        }
    }

    pub(crate) fn save_ui_state(&self) {
        let _ = self
            .store
            .set_ui_state(crate::app::UI_BOARD, &self.board.slug);
        let _ = self.store.set_ui_state(UI_FOCUS, &self.focus.to_string());
        if let Some(c) = self.selected_card() {
            let _ = self.store.set_ui_state(UI_SELECTED, &c.key);
        }
    }
}

fn human_intervention_kind(value: Option<&str>) -> Option<&str> {
    value
        .and_then(|v| HumanIntervention::parse(v).ok().flatten())
        .map(HumanIntervention::as_str)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NextWorkKind {
    Executable,
    Review,
    Decision,
    HumanExecution,
    MissingContext,
    DependencyBlocked,
    Blocked,
    Claimed,
}

impl NextWorkKind {
    fn rank(self) -> u8 {
        match self {
            Self::Executable => 0,
            Self::Review => 1,
            Self::Decision => 2,
            Self::HumanExecution => 3,
            Self::MissingContext => 4,
            Self::DependencyBlocked => 5,
            Self::Blocked => 6,
            Self::Claimed => 7,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::Review => "review",
            Self::Decision => "decision",
            Self::HumanExecution => "human-execution",
            Self::MissingContext => "missing-context",
            Self::DependencyBlocked => "dependency-blocked",
            Self::Blocked => "blocked",
            Self::Claimed => "claimed",
        }
    }
}

fn next_work_kind(card: &Card, readiness: &CardReadiness) -> Option<NextWorkKind> {
    if card.archived_at.is_some() || card.agent_state == "done" {
        return None;
    }
    if card.blocked_reason.is_some() {
        return Some(NextWorkKind::Blocked);
    }
    if claim_is_active(card) {
        return Some(NextWorkKind::Claimed);
    }
    if !readiness.ready {
        return Some(NextWorkKind::DependencyBlocked);
    }
    match card.human_gate() {
        Some(HumanIntervention::Review) => return Some(NextWorkKind::Review),
        Some(HumanIntervention::Decision) => return Some(NextWorkKind::Decision),
        Some(HumanIntervention::Execution) => return Some(NextWorkKind::HumanExecution),
        None => {}
    }
    if card.next_action.is_none() || card.acceptance_criteria.is_none() {
        return Some(NextWorkKind::MissingContext);
    }
    Some(NextWorkKind::Executable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_core::{BoardColumnTemplate, Store};

    fn card() -> Card {
        Card {
            id: "card".into(),
            board_id: "board".into(),
            column_id: "col".into(),
            key: "KB-1".into(),
            title: "task".into(),
            body: String::new(),
            agent_state: "open".into(),
            priority: 1,
            assignee: None,
            due_date: None,
            next_action: Some("do it".into()),
            blocked_reason: None,
            acceptance_criteria: Some("done".into()),
            handoff_note: None,
            last_verification: None,
            agent_weight: None,
            agent_effort: None,
            suggested_model: None,
            expected_tokens: None,
            human_intervention: None,
            claimed_by: None,
            claimed_at: None,
            lease_expires_at: None,
            position: 0.0,
            created_at: 0,
            updated_at: 0,
            archived_at: None,
        }
    }

    fn readiness(ready: bool) -> CardReadiness {
        CardReadiness {
            card_key: "KB-1".into(),
            ready,
            closed: false,
            blocked_by: Vec::new(),
        }
    }

    #[test]
    fn next_work_classification_prioritizes_actionable_states() {
        assert_eq!(
            next_work_kind(&card(), &readiness(true)),
            Some(NextWorkKind::Executable)
        );

        let mut review = card();
        review.human_intervention = Some("review".into());
        assert_eq!(
            next_work_kind(&review, &readiness(true)),
            Some(NextWorkKind::Review)
        );

        let mut missing = card();
        missing.next_action = None;
        assert_eq!(
            next_work_kind(&missing, &readiness(true)),
            Some(NextWorkKind::MissingContext)
        );

        assert_eq!(
            next_work_kind(&card(), &readiness(false)),
            Some(NextWorkKind::DependencyBlocked)
        );
    }

    #[test]
    fn resync_external_refreshes_board_metadata_and_columns() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();
        let mut app = App::new(store, board.clone()).unwrap();

        app.store
            .update_board_agent_context(&board.id, Some("Run checks before closing."))
            .unwrap();
        app.store.add_column(&board.id, "Review").unwrap();

        assert_eq!(app.board.agent_context, None);
        assert!(!app.columns.iter().any(|c| c.name == "Review"));

        app.resync_external().unwrap();

        assert_eq!(
            app.board.agent_context.as_deref(),
            Some("Run checks before closing.")
        );
        assert!(app.columns.iter().any(|c| c.name == "Review"));
    }

    #[test]
    fn resync_external_recovers_when_current_board_disappears() {
        let mut store = Store::open_in_memory().unwrap();
        // The Backlog board always exists in a real store and is the guaranteed
        // fallback target when the viewed board disappears.
        store.ensure_default_board().unwrap();
        let work = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();
        let mut app = App::new(store, work.clone()).unwrap();
        assert_eq!(app.board.id, work.id);

        // An external writer archives the board we're viewing while we sit in a
        // non-Normal mode that points at its now-gone structure.
        app.mode = Mode::ColumnManager;
        app.store.archive_board(&work.id).unwrap();

        app.resync_external().unwrap();

        // We fall back to a surviving board (the always-present Backlog), reset
        // the mode, and surface why the view changed.
        assert_ne!(app.board.id, work.id);
        assert_eq!(app.board.slug, kanban_core::PROTECTED_BOARD_SLUG);
        assert!(matches!(app.mode, Mode::Normal));
        assert!(app.status.contains("unavailable"));
    }
}
