use kanban_core::{card_is_stale, now_ms, Card, CardReadiness};
use std::collections::HashMap;

use crate::params::ListParams;

use super::super::super::queue::{classify_queue, QueueMode, QueueStatus};

pub(super) struct ListFilterContext<'a> {
    params: &'a ListParams,
    column_names: &'a HashMap<String, String>,
    queue: Option<QueueMode>,
    now: i64,
}

#[derive(Clone)]
pub(super) struct ListEntry {
    pub(super) column_name: String,
    pub(super) queue: Option<QueueMode>,
    pub(super) queue_status: QueueStatus,
}

pub(super) fn filter_context<'a>(
    params: &'a ListParams,
    column_names: &'a HashMap<String, String>,
    queue: Option<QueueMode>,
) -> ListFilterContext<'a> {
    ListFilterContext {
        params,
        column_names,
        queue,
        now: now_ms(),
    }
}

pub(super) fn list_entry(
    card: &Card,
    readiness: &CardReadiness,
    context: &ListFilterContext<'_>,
) -> Option<ListEntry> {
    let column_name = context
        .column_names
        .get(&card.column_id)
        .cloned()
        .unwrap_or_default();
    if !matches_basic_filters(card, &column_name, context.params) {
        return None;
    }
    let queue_status = classify_queue(card, context.now, readiness);
    if let Some(queue) = context.queue {
        if !queue.matches(queue_status) {
            return None;
        }
    }
    Some(ListEntry {
        column_name,
        queue: context.queue,
        queue_status,
    })
}

fn matches_basic_filters(card: &Card, column_name: &str, params: &ListParams) -> bool {
    if let Some(column) = &params.column {
        if column_name != column {
            return false;
        }
    }
    let agent_state_filter = params.agent_state.as_ref().or(params.status.as_ref());
    if let Some(agent_state) = agent_state_filter {
        if &card.agent_state != agent_state {
            return false;
        }
    }
    if let Some(stale) = params.stale {
        if card_is_stale(card) != stale {
            return false;
        }
    }
    if let Some(max) = params.agent_weight_max {
        if card.agent_weight.is_none_or(|weight| weight > max) {
            return false;
        }
    }
    if let Some(agent_effort) = params.agent_effort.as_deref() {
        if card.agent_effort.as_deref() != Some(agent_effort) {
            return false;
        }
    }
    if let Some(suggested_model) = params.suggested_model.as_deref() {
        if card.suggested_model.as_deref() != Some(suggested_model) {
            return false;
        }
    }
    if let Some(min) = params.expected_tokens_min {
        if card.expected_tokens.is_none_or(|tokens| tokens < min) {
            return false;
        }
    }
    if let Some(max) = params.expected_tokens_max {
        if card.expected_tokens.is_none_or(|tokens| tokens > max) {
            return false;
        }
    }
    if let Some(human_intervention) = params.human_intervention.as_deref() {
        let current = card.human_intervention.as_deref().unwrap_or("none");
        if current != human_intervention {
            return false;
        }
    }
    true
}
