use kanterm_core::{classify_work, Card, CardReadiness, HumanIntervention, WorkState};
use rmcp::ErrorData;

use crate::error::bad_param;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum QueueMode {
    Executable,
    Review,
    Blocked,
    Claimed,
    MissingContext,
    DependencyBlocked,
    Human,
}

impl QueueMode {
    pub(super) fn parse(value: &str) -> Result<Self, ErrorData> {
        match value {
            "executable" => Ok(Self::Executable),
            "review" => Ok(Self::Review),
            "blocked" => Ok(Self::Blocked),
            "claimed" => Ok(Self::Claimed),
            "missing_context" => Ok(Self::MissingContext),
            "dependency_blocked" => Ok(Self::DependencyBlocked),
            "human" => Ok(Self::Human),
            _ => Err(bad_param(
                "queue must be executable, review, blocked, claimed, missing_context, dependency_blocked, or human",
            )),
        }
    }

    pub(super) fn matches(self, status: QueueStatus) -> bool {
        match self {
            QueueMode::Executable => status == QueueStatus::Executable,
            QueueMode::Review => status == QueueStatus::ReviewRequired,
            QueueMode::Blocked => status == QueueStatus::Blocked,
            QueueMode::Claimed => status == QueueStatus::Claimed,
            QueueMode::MissingContext => status == QueueStatus::MissingContext,
            QueueMode::DependencyBlocked => status == QueueStatus::DependencyBlocked,
            QueueMode::Human => matches!(
                status,
                QueueStatus::ReviewRequired
                    | QueueStatus::HumanDecision
                    | QueueStatus::HumanExecution
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum QueueStatus {
    Executable,
    ReviewRequired,
    HumanDecision,
    HumanExecution,
    Blocked,
    Claimed,
    MissingContext,
    DependencyBlocked,
    Closed,
}

pub(super) fn classify_queue(card: &Card, now: i64, readiness: &CardReadiness) -> QueueStatus {
    match classify_work(card, readiness, now) {
        WorkState::Closed => QueueStatus::Closed,
        WorkState::Blocked => QueueStatus::Blocked,
        WorkState::Claimed => QueueStatus::Claimed,
        WorkState::DependencyBlocked => QueueStatus::DependencyBlocked,
        WorkState::Human(HumanIntervention::Review) => QueueStatus::ReviewRequired,
        WorkState::Human(HumanIntervention::Decision) => QueueStatus::HumanDecision,
        WorkState::Human(HumanIntervention::Execution) => QueueStatus::HumanExecution,
        WorkState::MissingContext => QueueStatus::MissingContext,
        WorkState::Executable => QueueStatus::Executable,
    }
}

pub(super) fn dependency_suffix(status: QueueStatus, readiness: &CardReadiness) -> String {
    if status != QueueStatus::DependencyBlocked || readiness.blocked_by.is_empty() {
        return String::new();
    }
    let blockers = readiness
        .blocked_by
        .iter()
        .map(|b| b.key.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(" [blocked_by:{blockers}]")
}

pub(super) fn queue_suffix(mode: Option<QueueMode>, status: QueueStatus) -> String {
    if mode.is_none() {
        return String::new();
    }
    let label = match status {
        QueueStatus::Executable => "executable",
        QueueStatus::ReviewRequired => "review",
        QueueStatus::HumanDecision => "human-decision",
        QueueStatus::HumanExecution => "human-execution",
        QueueStatus::Blocked => "blocked",
        QueueStatus::Claimed => "claimed",
        QueueStatus::MissingContext => "missing-context",
        QueueStatus::DependencyBlocked => "dependency-blocked",
        QueueStatus::Closed => "closed",
    };
    format!(" [queue:{label}]")
}
