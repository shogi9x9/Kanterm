use crate::app::App;
use kanban_core::{classify_work, now_ms, Card, CardReadiness, HumanIntervention, WorkState};

impl App {
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
        let now = now_ms();
        for col in 0..self.columns.len() {
            for (pos, card) in self.column_cards(col).iter().enumerate() {
                let Ok(readiness) = self.store.card_readiness(&self.board.id, &card.key) else {
                    continue;
                };
                if let Some(kind) = next_work_kind(card, &readiness, now) {
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

fn next_work_kind(card: &Card, readiness: &CardReadiness, now: i64) -> Option<NextWorkKind> {
    match classify_work(card, readiness, now) {
        WorkState::Closed => None,
        WorkState::Blocked => Some(NextWorkKind::Blocked),
        WorkState::Claimed => Some(NextWorkKind::Claimed),
        WorkState::DependencyBlocked => Some(NextWorkKind::DependencyBlocked),
        WorkState::Human(HumanIntervention::Review) => Some(NextWorkKind::Review),
        WorkState::Human(HumanIntervention::Decision) => Some(NextWorkKind::Decision),
        WorkState::Human(HumanIntervention::Execution) => Some(NextWorkKind::HumanExecution),
        WorkState::MissingContext => Some(NextWorkKind::MissingContext),
        WorkState::Executable => Some(NextWorkKind::Executable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let now = now_ms();
        assert_eq!(
            next_work_kind(&card(), &readiness(true), now),
            Some(NextWorkKind::Executable)
        );

        let mut review = card();
        review.human_intervention = Some("review".into());
        assert_eq!(
            next_work_kind(&review, &readiness(true), now),
            Some(NextWorkKind::Review)
        );

        let mut missing = card();
        missing.next_action = None;
        assert_eq!(
            next_work_kind(&missing, &readiness(true), now),
            Some(NextWorkKind::MissingContext)
        );

        assert_eq!(
            next_work_kind(&card(), &readiness(false), now),
            Some(NextWorkKind::DependencyBlocked)
        );
    }
}
