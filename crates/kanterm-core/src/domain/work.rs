use crate::now_ms;

use super::{Card, CardReadiness};

pub const STALE_CARD_MS: i64 = 30 * 24 * 60 * 60 * 1000;

pub fn card_is_stale(card: &Card) -> bool {
    now_ms().saturating_sub(card.updated_at) > STALE_CARD_MS
}

/// A human-intervention gate on a card. The stored and wire representation is the
/// lowercase variant name; an unset value or the literal `"none"` means "no gate"
/// and is modelled as `Option::None` rather than a dedicated variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanIntervention {
    Review,
    Decision,
    Execution,
}

impl HumanIntervention {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Review => "review",
            Self::Decision => "decision",
            Self::Execution => "execution",
        }
    }

    /// Parse a stored or wire value. Empty/whitespace and `"none"` yield
    /// `Ok(None)` (gate cleared); `review`/`decision`/`execution` yield
    /// `Ok(Some(_))`; anything else is rejected with the canonical message.
    pub fn parse(value: &str) -> Result<Option<Self>, String> {
        match value.trim() {
            "" | "none" => Ok(None),
            "review" => Ok(Some(Self::Review)),
            "decision" => Ok(Some(Self::Decision)),
            "execution" => Ok(Some(Self::Execution)),
            _ => Err("human_intervention must be none, review, decision, or execution".to_string()),
        }
    }
}

impl Card {
    /// The active human-intervention gate, or `None` when unset, `"none"`, or an
    /// unrecognised legacy value. Use this for any branching on the gate so a new
    /// variant forces every match to be revisited.
    pub fn human_gate(&self) -> Option<HumanIntervention> {
        self.human_intervention
            .as_deref()
            .and_then(|value| HumanIntervention::parse(value).ok().flatten())
    }

    /// Whether the card is terminal: completed (`agent_state == "done"`) or
    /// archived. Note this is deliberately *not* used for upstream dependency
    /// blocking, where an archived-but-unfinished upstream still blocks.
    pub fn is_closed(&self) -> bool {
        self.agent_state == "done" || self.archived_at.is_some()
    }

    /// Whether the card has a live agent claim: owned and within its lease.
    pub fn claim_is_active(&self, now: i64) -> bool {
        self.claimed_by.is_some()
            && self
                .lease_expires_at
                .is_some_and(|expires_at| expires_at > now)
    }
}

/// Work-queue classification of a card: where it sits in the execution pipeline.
/// Adapters map this to their own labels/filters; keep the precedence here as the
/// single source so the TUI and MCP can't drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    Closed,
    Blocked,
    Claimed,
    DependencyBlocked,
    Human(HumanIntervention),
    MissingContext,
    Executable,
}

/// Classify a card for the work queue. Precedence:
/// closed -> blocked -> claimed -> dependency-blocked -> human gate -> missing -> executable.
pub fn classify_work(card: &Card, readiness: &CardReadiness, now: i64) -> WorkState {
    if card.is_closed() {
        return WorkState::Closed;
    }
    if card.blocked_reason.is_some() {
        return WorkState::Blocked;
    }
    if card.claim_is_active(now) {
        return WorkState::Claimed;
    }
    if !readiness.ready {
        return WorkState::DependencyBlocked;
    }
    if let Some(gate) = card.human_gate() {
        return WorkState::Human(gate);
    }
    if card.next_action.is_none() || card.acceptance_criteria.is_none() {
        return WorkState::MissingContext;
    }
    WorkState::Executable
}

/// Dependency-graph node classification of a card. Distinct precedence from
/// [`WorkState`]: a live claim and human gate are surfaced before the
/// blocked/dependency state, matching the graph view's emphasis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNodeState {
    Done,
    Running,
    Human(HumanIntervention),
    DependencyBlocked,
    Blocked,
    MissingContext,
    Ready,
}

impl GraphNodeState {
    /// The compact status token shown in the dependency graph.
    pub fn label(self) -> String {
        match self {
            Self::Done => "done".into(),
            Self::Running => "running".into(),
            Self::Human(gate) => format!("human:{}", gate.as_str()),
            Self::DependencyBlocked => "dep-blocked".into(),
            Self::Blocked => "blocked".into(),
            Self::MissingContext => "missing".into(),
            Self::Ready => "ready".into(),
        }
    }
}

/// Classify a card for the dependency graph. Precedence:
/// done -> running -> human gate -> dependency-blocked -> blocked -> missing -> ready.
pub fn classify_graph_node(card: &Card, readiness: &CardReadiness, now: i64) -> GraphNodeState {
    if card.is_closed() {
        return GraphNodeState::Done;
    }
    if card.claim_is_active(now) {
        return GraphNodeState::Running;
    }
    if let Some(gate) = card.human_gate() {
        return GraphNodeState::Human(gate);
    }
    if !readiness.ready {
        return GraphNodeState::DependencyBlocked;
    }
    if card.blocked_reason.is_some() {
        return GraphNodeState::Blocked;
    }
    if card.next_action.is_none() || card.acceptance_criteria.is_none() {
        return GraphNodeState::MissingContext;
    }
    GraphNodeState::Ready
}
