use anyhow::Result;
use kanterm_core::{classify_work, now_ms, Board, Card, HumanIntervention, WorkState};
use std::collections::HashMap;

use super::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DashboardGroup {
    Running,
    Human,
    Ready,
    Blocked,
    Waiting,
    Missing,
}

impl DashboardGroup {
    pub(crate) const ALL: [Self; 6] = [
        Self::Running,
        Self::Human,
        Self::Ready,
        Self::Blocked,
        Self::Waiting,
        Self::Missing,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Human => "HUMAN",
            Self::Ready => "READY",
            Self::Blocked => "BLOCKED",
            Self::Waiting => "WAITING",
            Self::Missing => "MISSING",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Running => 0,
            Self::Human => 1,
            Self::Ready => 2,
            Self::Blocked => 3,
            Self::Waiting => 4,
            Self::Missing => 5,
        }
    }
}

pub(crate) const FLOW_GROUPS: [DashboardGroup; 6] = [
    DashboardGroup::Missing,
    DashboardGroup::Waiting,
    DashboardGroup::Blocked,
    DashboardGroup::Human,
    DashboardGroup::Ready,
    DashboardGroup::Running,
];

impl From<WorkState> for DashboardGroup {
    fn from(state: WorkState) -> Self {
        match state {
            WorkState::Claimed => Self::Running,
            WorkState::Human(_) => Self::Human,
            WorkState::Executable => Self::Ready,
            WorkState::Blocked => Self::Blocked,
            WorkState::DependencyBlocked => Self::Waiting,
            WorkState::MissingContext => Self::Missing,
            WorkState::Closed => unreachable!("closed cards are omitted from the dashboard"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DashboardItem {
    pub(crate) board: Board,
    pub(crate) card: Card,
    pub(crate) state: WorkState,
    pub(crate) group: DashboardGroup,
    pub(crate) blocked_by: Vec<String>,
    board_order: usize,
    card_order: usize,
}

#[derive(Clone)]
pub(crate) struct TimelineItem {
    pub(crate) item: DashboardItem,
    /// Zero-based topological stage. `None` means an external blocker or cycle
    /// kept the card out of the executable stage plan.
    pub(crate) stage: Option<usize>,
}

impl DashboardItem {
    pub(crate) fn state_label(&self) -> &'static str {
        match self.state {
            WorkState::Closed => "closed",
            WorkState::Blocked => "blocked",
            WorkState::Claimed => "running",
            WorkState::DependencyBlocked => "waiting",
            WorkState::Human(HumanIntervention::Review) => "review",
            WorkState::Human(HumanIntervention::Decision) => "decision",
            WorkState::Human(HumanIntervention::Execution) => "execution",
            WorkState::MissingContext => "missing",
            WorkState::Executable => "ready",
        }
    }

    pub(crate) fn signal(&self, now: i64) -> String {
        match self.state {
            WorkState::Claimed => {
                let owner = self.card.claimed_by.as_deref().unwrap_or("unknown");
                match self.card.lease_expires_at {
                    Some(expires_at) => format!("{owner} · {}", lease_label(expires_at - now)),
                    None => owner.to_string(),
                }
            }
            WorkState::Human(gate) => join_signal(gate.as_str(), self.card.next_action.as_deref()),
            WorkState::Executable => compact(self.card.next_action.as_deref().unwrap_or("ready")),
            WorkState::Blocked => compact(
                self.card
                    .blocked_reason
                    .as_deref()
                    .unwrap_or("blocked without a reason"),
            ),
            WorkState::DependencyBlocked => {
                if self.blocked_by.is_empty() {
                    "waiting on dependencies".into()
                } else {
                    format!("blocked by {}", self.blocked_by.join(", "))
                }
            }
            WorkState::MissingContext => match (
                self.card.next_action.is_some(),
                self.card.acceptance_criteria.is_some(),
            ) {
                (false, false) => "needs next action + acceptance".into(),
                (false, true) => "needs next action".into(),
                (true, false) => "needs acceptance criteria".into(),
                (true, true) => "context incomplete".into(),
            },
            WorkState::Closed => String::new(),
        }
    }
}

#[derive(Default)]
pub(crate) struct DashboardCounts {
    running: usize,
    human: usize,
    ready: usize,
    blocked: usize,
    waiting: usize,
    missing: usize,
}

impl DashboardCounts {
    pub(crate) fn from_items(items: &[DashboardItem]) -> Self {
        let mut counts = Self::default();
        for item in items {
            match item.group {
                DashboardGroup::Running => counts.running += 1,
                DashboardGroup::Human => counts.human += 1,
                DashboardGroup::Ready => counts.ready += 1,
                DashboardGroup::Blocked => counts.blocked += 1,
                DashboardGroup::Waiting => counts.waiting += 1,
                DashboardGroup::Missing => counts.missing += 1,
            }
        }
        counts
    }

    pub(crate) fn get(&self, group: DashboardGroup) -> usize {
        match group {
            DashboardGroup::Running => self.running,
            DashboardGroup::Human => self.human,
            DashboardGroup::Ready => self.ready,
            DashboardGroup::Blocked => self.blocked,
            DashboardGroup::Waiting => self.waiting,
            DashboardGroup::Missing => self.missing,
        }
    }
}

impl App {
    pub(crate) fn execution_dashboard_items(&self) -> Result<Vec<DashboardItem>> {
        let now = now_ms();
        let mut items = Vec::new();
        for (board_order, board) in self.boards.iter().enumerate() {
            for (card_order, card) in self.store.cards(&board.id)?.into_iter().enumerate() {
                let readiness = self.store.card_readiness(&board.id, &card.key)?;
                let state = classify_work(&card, &readiness, now);
                if state == WorkState::Closed {
                    continue;
                }
                items.push(DashboardItem {
                    board: board.clone(),
                    card,
                    state,
                    group: state.into(),
                    blocked_by: readiness
                        .blocked_by
                        .into_iter()
                        .map(|card| card.key)
                        .collect(),
                    board_order,
                    card_order,
                });
            }
        }
        items.sort_by_key(|item| {
            (
                item.group.rank(),
                std::cmp::Reverse(item.card.priority),
                item.board_order,
                item.card_order,
            )
        });
        Ok(items)
    }

    pub(crate) fn execution_timeline_items(&self) -> Result<(Vec<TimelineItem>, usize)> {
        let mut stage_by_card = HashMap::new();
        let mut max_stages = 0;
        for board in &self.boards {
            let plan = self.store.dependency_stage_plan(&board.id)?;
            max_stages = max_stages.max(plan.ready_stages.len());
            for (stage, keys) in plan.ready_stages.into_iter().enumerate() {
                for key in keys {
                    stage_by_card.insert((board.id.clone(), key), stage);
                }
            }
        }

        let mut timeline = self
            .execution_dashboard_items()?
            .into_iter()
            .map(|item| TimelineItem {
                stage: stage_by_card
                    .get(&(item.board.id.clone(), item.card.key.clone()))
                    .copied(),
                item,
            })
            .collect::<Vec<_>>();
        timeline.sort_by_key(|entry| {
            (
                entry.item.board_order,
                entry.stage.unwrap_or(max_stages),
                entry.item.card_order,
            )
        });
        Ok((timeline, max_stages))
    }

    pub(crate) fn execution_items_for_group(
        &self,
        group: DashboardGroup,
    ) -> Result<Vec<DashboardItem>> {
        Ok(self
            .execution_dashboard_items()?
            .into_iter()
            .filter(|item| item.group == group)
            .collect())
    }
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn join_signal(prefix: &str, value: Option<&str>) -> String {
    match value.map(compact).filter(|value| !value.is_empty()) {
        Some(value) => format!("{prefix} · {value}"),
        None => prefix.to_string(),
    }
}

fn lease_label(remaining_ms: i64) -> String {
    if remaining_ms <= 0 {
        return "lease expired".into();
    }
    let minutes = (remaining_ms + 59_999) / 60_000;
    if minutes < 60 {
        format!("lease {minutes}m")
    } else {
        format!("lease {}h{}m", minutes / 60, minutes % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanterm_core::{BoardColumnTemplate, CardPatch, Store};

    #[test]
    fn dashboard_collects_active_work_across_boards() {
        let mut store = Store::open_in_memory().unwrap();
        let backlog = store.ensure_default_board().unwrap();
        let ready = store
            .create_card(&backlog.id, None, "ready", "", "test")
            .unwrap();
        store
            .update_card(
                &backlog.id,
                &ready.key,
                &CardPatch {
                    next_action: Some("run tests".into()),
                    acceptance_criteria: Some("tests pass".into()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        let work = store
            .create_board("Work", BoardColumnTemplate::Workflow)
            .unwrap();
        store
            .create_card(&work.id, None, "missing", "", "test")
            .unwrap();

        let app = App::new(store, backlog).unwrap();
        let items = app.execution_dashboard_items().unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].group, DashboardGroup::Ready);
        assert_eq!(items[0].board.slug, "backlog");
        assert_eq!(items[1].group, DashboardGroup::Missing);
        assert_eq!(items[1].board.slug, work.slug);
    }

    #[test]
    fn dashboard_is_the_first_tui_view() {
        let mut store = Store::open_in_memory().unwrap();
        let backlog = store.ensure_default_board().unwrap();
        let app = App::new(store, backlog).unwrap();

        assert!(matches!(
            app.mode,
            crate::mode::Mode::ExecutionDashboard {
                view: crate::mode::ExecutionDashboardView::List,
                cursor: 0,
                focus: 0,
            }
        ));
    }

    #[test]
    fn lease_labels_are_compact() {
        assert_eq!(lease_label(1), "lease 1m");
        assert_eq!(lease_label(60 * 60 * 1000), "lease 1h0m");
        assert_eq!(lease_label(0), "lease expired");
    }

    #[test]
    fn dependency_blocked_work_has_its_own_waiting_group() {
        assert_eq!(
            DashboardGroup::from(WorkState::DependencyBlocked),
            DashboardGroup::Waiting
        );
        assert_eq!(
            DashboardGroup::from(WorkState::Blocked),
            DashboardGroup::Blocked
        );
    }

    #[test]
    fn timeline_uses_dependency_stages_across_active_cards() {
        let mut store = Store::open_in_memory().unwrap();
        let backlog = store.ensure_default_board().unwrap();
        let first = store
            .create_card(&backlog.id, None, "first", "", "test")
            .unwrap();
        let second = store
            .create_card(&backlog.id, None, "second", "", "test")
            .unwrap();
        store
            .set_card_dependencies(
                &backlog.id,
                &second.key,
                std::slice::from_ref(&first.key),
                "test",
            )
            .unwrap();

        let app = App::new(store, backlog).unwrap();
        let (items, max_stages) = app.execution_timeline_items().unwrap();

        assert_eq!(max_stages, 2);
        assert_eq!(items[0].item.card.key, first.key);
        assert_eq!(items[0].stage, Some(0));
        assert_eq!(items[1].item.card.key, second.key);
        assert_eq!(items[1].stage, Some(1));
    }
}
