use kanterm_core::{AgentHandoff, HandoffDraft, HandoffStatusPatch, Store};
use rmcp::ErrorData;

use crate::error::internal;
use crate::lookup::resolve_board;
use crate::params::{
    ClaimHandoffParams, CompleteHandoffParams, ListHandoffsParams, SendHandoffParams,
};

pub(crate) fn send_handoff(
    store: &mut Store,
    default_board_id: &str,
    p: SendHandoffParams,
) -> Result<String, ErrorData> {
    let board_id = if p.board.is_some() || p.card.is_some() {
        Some(resolve_board(store, default_board_id, p.board.as_deref())?)
    } else {
        None
    };
    let handoff = store
        .create_handoff(&HandoffDraft {
            from_agent: p.from_agent,
            to_agent: p.to_agent,
            board_id,
            card_key: p.card,
            subject: p.subject,
            body: p.body,
        })
        .map_err(internal)?;
    Ok(format!(
        "handoff_sent: {}\nstatus: {}\nto_agent: {}\nsubject: {}",
        handoff.id, handoff.status, handoff.to_agent, handoff.subject
    ))
}

pub(crate) fn list_handoffs(store: &Store, p: ListHandoffsParams) -> Result<String, ErrorData> {
    let handoffs = store
        .list_handoffs(
            p.for_agent.as_deref(),
            p.include_closed.unwrap_or(false),
            p.limit.unwrap_or(20),
        )
        .map_err(internal)?;
    if handoffs.is_empty() {
        return Ok("no handoffs".into());
    }
    Ok(handoffs
        .iter()
        .map(render_handoff_line)
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(crate) fn claim_handoff(store: &mut Store, p: ClaimHandoffParams) -> Result<String, ErrorData> {
    let handoff = store
        .claim_handoff(
            &p.id,
            &p.claimant,
            Some(p.claim_token.as_str()),
            p.lease_minutes,
        )
        .map_err(internal)?;
    Ok(format!(
        "handoff_claimed: {}\nclaimed_by: {}\nlease_expires_at: {}\nsubject: {}\nbody:\n{}",
        handoff.id,
        handoff.claimed_by.as_deref().unwrap_or("-"),
        handoff.lease_expires_at.unwrap_or_default(),
        handoff.subject,
        handoff.body
    ))
}

pub(crate) fn complete_handoff(
    store: &mut Store,
    p: CompleteHandoffParams,
) -> Result<String, ErrorData> {
    let handoff = store
        .update_handoff_status(
            &p.id,
            &p.claimant,
            Some(p.claim_token.as_str()),
            &HandoffStatusPatch {
                status: p.status,
                note: p.note,
            },
        )
        .map_err(internal)?;
    Ok(format!(
        "handoff_updated: {}\nstatus: {}",
        handoff.id, handoff.status
    ))
}

fn render_handoff_line(h: &AgentHandoff) -> String {
    let card = h
        .card_key
        .as_deref()
        .map(|key| format!(" card:{key}"))
        .unwrap_or_default();
    let claim = h
        .claimed_by
        .as_deref()
        .map(|agent| format!(" claimed:{agent}"))
        .unwrap_or_default();
    format!(
        "{} [{}] from:{} to:{}{}{} {}",
        h.id, h.status, h.from_agent, h.to_agent, card, claim, h.subject
    )
}
