use kanterm_core::{card_is_stale, priority_label, Card, CardReadiness, Label};
use std::collections::HashMap;

use crate::render::{claim_suffix, due_suffix, execution_suffix, label_suffix, priority};

use super::super::super::queue::{dependency_suffix, queue_suffix};
use super::filters::ListEntry;

pub(super) fn format_entry(
    card: &Card,
    readiness: &CardReadiness,
    labels: &HashMap<String, Vec<Label>>,
    entry: ListEntry,
    ranked: bool,
) -> ((i64, i64, i64, i64), String, String) {
    let stale = if card_is_stale(card) { " [stale]" } else { "" };
    let workflow = if card.blocked_reason.is_some() {
        " [blocked]"
    } else if card.next_action.is_some() {
        " [next]"
    } else {
        ""
    };
    let rank = rank_key(card);
    let rank_reason = if ranked {
        rank_reason(card)
    } else {
        String::new()
    };
    (
        rank,
        card.key.clone(),
        format!(
            "{}  [{}] ({}) {}{}{}{}{}{}{}{}{}{}",
            card.key,
            entry.column_name,
            priority(card),
            card.title,
            due_suffix(card),
            label_suffix(labels, &card.id),
            workflow,
            claim_suffix(card),
            stale,
            execution_suffix(card),
            dependency_suffix(entry.queue_status, readiness),
            queue_suffix(entry.queue, entry.queue_status),
            rank_reason,
        ),
    )
}

fn rank_key(card: &Card) -> (i64, i64, i64, i64) {
    (
        -card.priority,
        card.agent_weight.unwrap_or(3),
        card.expected_tokens.unwrap_or(i64::MAX),
        card.updated_at,
    )
}

fn rank_reason(card: &Card) -> String {
    let weight = card
        .agent_weight
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".into());
    let tokens = card
        .expected_tokens
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".into());
    format!(
        " [rank:priority={} weight={} tokens={}]",
        priority_label(card.priority),
        weight,
        tokens
    )
}
