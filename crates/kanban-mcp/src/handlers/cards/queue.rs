use kanban_core::{Card, CardReadiness};
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
    if card.archived_at.is_some() || card.agent_state == "done" {
        return QueueStatus::Closed;
    }
    if card.blocked_reason.is_some() {
        return QueueStatus::Blocked;
    }
    if card
        .lease_expires_at
        .is_some_and(|expires_at| card.claimed_by.is_some() && expires_at > now)
    {
        return QueueStatus::Claimed;
    }
    if !readiness.ready {
        return QueueStatus::DependencyBlocked;
    }
    match card.human_intervention.as_deref().unwrap_or("none") {
        "review" => return QueueStatus::ReviewRequired,
        "decision" => return QueueStatus::HumanDecision,
        "execution" => return QueueStatus::HumanExecution,
        _ => {}
    }
    if card.acceptance_criteria.is_none() || card.next_action.is_none() {
        return QueueStatus::MissingContext;
    }
    QueueStatus::Executable
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
