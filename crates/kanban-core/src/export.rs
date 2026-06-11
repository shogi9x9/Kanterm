use anyhow::Result;

use crate::{priority_label, Card, Store};

/// Pretty JSON snapshot of a board (columns -> cards -> labels).
pub(crate) fn export_json(store: &Store, board_id: &str) -> Result<String> {
    let board = store.board_by_id_or_slug(board_id)?;
    let cols = store.columns(board_id)?;
    let cards = store.cards(board_id)?;
    let labels = store.labels_by_card(board_id)?;
    let columns: Vec<_> = cols
        .iter()
        .map(|col| {
            let in_col: Vec<_> = cards
                .iter()
                .filter(|c| c.column_id == col.id)
                .map(|c| {
                    serde_json::json!({
                        "key": c.key,
                        "title": c.title,
                        "body": c.body,
                        "agent_state": c.agent_state,
                        "priority": priority_label(c.priority),
                        "assignee": c.assignee,
                        "next_action": c.next_action,
                        "blocked_reason": c.blocked_reason,
                        "acceptance_criteria": c.acceptance_criteria,
                        "handoff_note": c.handoff_note,
                        "last_verification": c.last_verification,
                        "agent_weight": c.agent_weight,
                        "agent_effort": c.agent_effort,
                        "suggested_model": c.suggested_model,
                        "expected_tokens": c.expected_tokens,
                        "human_intervention": c.human_intervention,
                        "claimed_by": c.claimed_by,
                        "claimed_at": c.claimed_at,
                        "lease_expires_at": c.lease_expires_at,
                        "labels": labels.get(&c.id).map(|ls| ls.iter().map(|l| &l.name).collect::<Vec<_>>()).unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({ "name": col.name, "cards": in_col })
        })
        .collect();
    let doc = serde_json::json!({
        "board": board_id,
        "agent_context": board.agent_context,
        "columns": columns
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// Markdown snapshot of a board, suitable for committing to git.
pub(crate) fn export_markdown(store: &Store, board_id: &str) -> Result<String> {
    let board = store.board_by_id_or_slug(board_id)?;
    let cols = store.columns(board_id)?;
    let cards = store.cards(board_id)?;
    let labels = store.labels_by_card(board_id)?;
    let mut out = String::from("# Kanban\n\n");
    if let Some(context) = board.agent_context.as_deref() {
        out.push_str("## Board Agent Context\n\n");
        out.push_str(context);
        out.push_str("\n\n");
    }
    for col in &cols {
        let in_col: Vec<&Card> = cards.iter().filter(|c| c.column_id == col.id).collect();
        out.push_str(&format!("## {} ({})\n\n", col.name, in_col.len()));
        for c in in_col {
            let tags = labels
                .get(&c.id)
                .map(|ls| {
                    ls.iter()
                        .map(|l| format!("`{}`", l.name))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            out.push_str(&format!("- **{}** {} {}\n", c.key, c.title, tags));
        }
        out.push('\n');
    }
    Ok(out)
}
