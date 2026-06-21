use std::collections::HashMap;

use kanban_core::{
    format_date, now_ms, priority_badge, today_start_ms, ActivityLog, Card, Label, Memory,
};

/// Render a card's due date as " due:YYYY-MM-DD", flagged with "!" when overdue.
pub(crate) fn due_suffix(card: &Card) -> String {
    match card.due_date {
        Some(ms) => {
            let overdue = ms < today_start_ms();
            format!(
                "  {}due:{}",
                if overdue { "!" } else { "" },
                format_date(ms)
            )
        }
        None => String::new(),
    }
}

pub(crate) fn complete_note_body(base: &str, note: &str) -> String {
    if base.trim().is_empty() {
        format!("[completion note] {note}")
    } else {
        format!("{base}\n[completion note] {note}")
    }
}

fn claim_is_active(card: &Card) -> bool {
    matches!(card.lease_expires_at, Some(expires_at) if expires_at > now_ms())
}

pub(crate) fn claim_suffix(card: &Card) -> String {
    match card.claimed_by.as_deref() {
        Some(claimed_by) if claim_is_active(card) => format!(" [claimed:{claimed_by}]"),
        Some(claimed_by) => format!(" [claim-expired:{claimed_by}]"),
        None => String::new(),
    }
}

pub(crate) fn claim_detail(card: &Card) -> String {
    match (card.claimed_by.as_deref(), card.lease_expires_at) {
        (Some(claimed_by), Some(expires_at)) if expires_at > now_ms() => {
            format!("{claimed_by} until lease_expires_at={expires_at}")
        }
        (Some(claimed_by), Some(expires_at)) => {
            format!("expired {claimed_by} at lease_expires_at={expires_at}")
        }
        (Some(claimed_by), None) => format!("{claimed_by} without lease"),
        _ => "-".into(),
    }
}

/// Render a card's labels as a trailing " [a, b]" tag, or "" if none.
pub(crate) fn label_suffix(labels: &HashMap<String, Vec<Label>>, card_id: &str) -> String {
    match labels.get(card_id) {
        Some(ls) if !ls.is_empty() => {
            format!(
                "  [{}]",
                ls.iter()
                    .map(|l| l.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        _ => String::new(),
    }
}

pub(crate) fn priority(card: &Card) -> &'static str {
    priority_badge(card.priority)
}

/// Render a card's agent_weight/human_intervention as a trailing " [..]" tag for
/// the board overview, or "" if neither is set.
pub(crate) fn board_execution_suffix(c: &Card) -> String {
    let mut parts = Vec::new();
    if let Some(weight) = c.agent_weight {
        parts.push(format!("w:{weight}"));
    }
    if let Some(human) = c.human_intervention.as_deref() {
        if human != "none" {
            parts.push(format!("human:{human}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(" "))
    }
}

/// Render a card's full execution metadata as a trailing " [..]" tag for the
/// card list, or "" if none is set.
pub(crate) fn execution_suffix(c: &Card) -> String {
    let mut parts = Vec::new();
    if let Some(weight) = c.agent_weight {
        parts.push(format!("w:{weight}"));
    }
    if let Some(effort) = c.agent_effort.as_deref() {
        parts.push(format!("effort:{effort}"));
    }
    if let Some(model) = c.suggested_model.as_deref() {
        parts.push(format!("model:{model}"));
    }
    if let Some(tokens) = c.expected_tokens {
        parts.push(format!("tokens:{tokens}"));
    }
    if let Some(human) = c.human_intervention.as_deref() {
        if human != "none" {
            parts.push(format!("human:{human}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(" "))
    }
}

pub(crate) fn activity_lines(logs: &[ActivityLog]) -> String {
    if logs.is_empty() {
        return "-".to_string();
    }
    logs.iter()
        .map(|log| {
            let detail = serde_json::from_str::<serde_json::Value>(&log.payload_json)
                .ok()
                .and_then(|v| render_activity_payload(&log.action, &v))
                .unwrap_or_else(|| log.payload_json.clone());
            format!(
                "- {} {} {} {}",
                log.created_at, log.actor, log.action, detail
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_activity_payload(action: &str, payload: &serde_json::Value) -> Option<String> {
    if action == "move_board" {
        let old_key = payload.get("old_key")?.as_str()?;
        let new_key = payload.get("new_key")?.as_str()?;
        let source = payload.get("source_board")?;
        let destination = payload.get("destination_board")?;
        return Some(format!(
            "{old_key} -> {new_key}; {} ({}) -> {} ({})",
            source.get("name")?.as_str()?,
            source.get("slug")?.as_str()?,
            destination.get("name")?.as_str()?,
            destination.get("slug")?.as_str()?,
        ));
    }
    if action == "execution_note" {
        return payload
            .get("note")
            .and_then(|note| note.as_str())
            .map(str::to_string);
    }
    payload
        .get("detail")
        .and_then(|detail| detail.as_str())
        .map(str::to_string)
}

pub(crate) fn execution_note_lines(logs: &[ActivityLog]) -> String {
    let lines: Vec<String> = logs
        .iter()
        .filter(|log| log.action == "execution_note")
        .filter_map(|log| {
            let note = serde_json::from_str::<serde_json::Value>(&log.payload_json)
                .ok()?
                .get("note")?
                .as_str()?
                .to_string();
            Some(format!("- {} {} {}", log.created_at, log.actor, note))
        })
        .collect();
    if lines.is_empty() {
        "-".to_string()
    } else {
        lines.join("\n")
    }
}

pub(crate) fn metadata_value(value: Option<&str>) -> &str {
    value.filter(|s| !s.trim().is_empty()).unwrap_or("-")
}

pub(crate) fn metadata_i64(value: Option<i64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string())
}

pub(crate) fn memory_lines(memories: &[Memory]) -> String {
    if memories.is_empty() {
        return "-".to_string();
    }
    memories
        .iter()
        .map(|m| {
            let snippet = m.body.lines().next().unwrap_or("");
            if snippet.is_empty() {
                format!("- {} [{}] {}", m.key, m.kind, m.title)
            } else {
                format!("- {} [{}] {} - {}", m.key, m.kind, m.title, snippet)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
