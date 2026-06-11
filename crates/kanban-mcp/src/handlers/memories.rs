use kanban_core::{format_date, Store};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::params::{RecallMemoriesParams, RecordMemoryParams};

pub(crate) fn record_memory(store: &mut Store, p: RecordMemoryParams) -> Result<String, ErrorData> {
    let m = store
        .record_memory(
            &p.title,
            p.body.as_deref().unwrap_or(""),
            p.kind.as_deref(),
            p.card.as_deref(),
        )
        .map_err(internal)?;
    Ok(format!("recorded {} [{}]: {}", m.key, m.kind, m.title))
}

pub(crate) fn recall_memories(
    store: &mut Store,
    p: RecallMemoriesParams,
) -> Result<String, ErrorData> {
    if let Some(key) = p.key.as_deref() {
        let m = store
            .memory_by_key(key)
            .map_err(internal)?
            .ok_or_else(|| bad_param(format!("no memory '{key}'")))?;
        store
            .mark_memories_recalled([m.key.as_str()])
            .map_err(internal)?;
        let mut out = format!(
            "{} [{}] ({}){}\n",
            m.key,
            m.kind,
            format_date(m.created_at),
            m.card_key
                .as_deref()
                .map(|c| format!("  card:{c}"))
                .unwrap_or_default()
        );
        out.push_str(&format!("# {}\n", m.title));
        if !m.body.is_empty() {
            out.push_str(&m.body);
            out.push('\n');
        }
        return Ok(out);
    }
    let limit = p.limit.unwrap_or(10).clamp(1, 100);
    let hits = store
        .recall_memories(
            p.query.as_deref(),
            p.card.as_deref(),
            p.kind.as_deref(),
            limit,
            false,
        )
        .map_err(internal)?;
    if hits.is_empty() {
        return Ok("no memories found".into());
    }
    let keys: Vec<&str> = hits.iter().map(|m| m.key.as_str()).collect();
    store.mark_memories_recalled(keys).map_err(internal)?;
    let lines: Vec<String> = hits
        .iter()
        .map(|m| {
            let snippet = m.body.lines().next().unwrap_or("");
            format!(
                "{} [{}] ({}){} {}{}",
                m.key,
                m.kind,
                format_date(m.created_at),
                m.card_key
                    .as_deref()
                    .map(|c| format!(" card:{c}"))
                    .unwrap_or_default(),
                m.title,
                if snippet.is_empty() {
                    String::new()
                } else {
                    format!(" — {snippet}")
                }
            )
        })
        .collect();
    Ok(lines.join("\n"))
}
