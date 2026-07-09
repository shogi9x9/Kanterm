use anyhow::{anyhow, Result};
use kanterm_core::{CardPatch, Store};

use super::args::AgentTaskArgs;
use crate::render::complete_note_body;

pub(super) fn complete_card(
    store: &mut Store,
    args: &AgentTaskArgs,
    agent: &str,
    output: &str,
) -> Result<()> {
    let board = store.board_by_id_or_slug(&args.board)?;
    let card = store
        .card_by_key(&board.id, &args.card)?
        .ok_or_else(|| anyhow!("no card '{}'", args.card))?;
    let note = args
        .complete_note
        .clone()
        .unwrap_or_else(|| format!("agent task completed by {agent}: {}", output.trim()));
    let patch = CardPatch {
        body: Some(complete_note_body(&card.body, note.trim())),
        agent_state: Some("done".into()),
        archived: Some(true),
        next_action: Some(String::new()),
        blocked_reason: Some(String::new()),
        handoff_note: Some(String::new()),
        ..Default::default()
    };
    store.update_card(&board.id, &args.card, &patch, agent)?;
    Ok(())
}
