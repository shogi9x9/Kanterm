use kanterm_core::{CardCreateDraft, Store};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::lookup::resolve_or_create_project_board;
use crate::params::{CreateBacklogCardParams, CreateCardsParams, CreateParams};

use super::super::plan_import::preflight_create_cards;

fn cleanup_created_board(store: &mut Store, board_id: &str) {
    let _ = store.archive_board(board_id);
    let _ = store.delete_board(board_id);
}

pub(crate) fn create_card(
    store: &mut Store,
    _default_board_id: &str,
    p: CreateParams,
) -> Result<String, ErrorData> {
    let destination = resolve_or_create_project_board(store, p.board.as_deref(), "create_card")?;
    let column = p.column.as_deref().unwrap_or("first column");
    let card = store
        .create_card(
            &destination.id,
            p.column.as_deref(),
            &p.title,
            p.body.as_deref().unwrap_or(""),
            "agent",
        )
        .map_err(internal)?;
    Ok(format!(
        "created {} in board '{}' (board: {}) column '{}'",
        card.key,
        destination.slug,
        if destination.created {
            "created"
        } else {
            "existing"
        },
        column
    ))
}

pub(crate) fn create_cards(
    store: &mut Store,
    _default_board_id: &str,
    p: CreateCardsParams,
) -> Result<String, ErrorData> {
    if p.cards.is_empty() {
        return Err(bad_param("cards must not be empty"));
    }
    let destination = resolve_or_create_project_board(store, p.board.as_deref(), "create_cards")?;
    if let Err(err) = preflight_create_cards(store, &destination.id, &p.cards) {
        if destination.created {
            cleanup_created_board(store, &destination.id);
        }
        return Err(err);
    }
    let drafts = p
        .cards
        .into_iter()
        .map(|item| CardCreateDraft {
            alias: item.alias,
            title: item.title,
            body: item.body.unwrap_or_default(),
            column: item.column,
            next_action: item.next_action,
            acceptance_criteria: item.acceptance_criteria,
            agent_weight: item.agent_weight,
            agent_effort: item.agent_effort,
            suggested_model: item.suggested_model,
            expected_tokens: item.expected_tokens,
            human_intervention: item.human_intervention,
            depends_on: item.depends_on.unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let created_cards = match store.create_cards_from_plan(&destination.id, &drafts, "agent") {
        Ok(cards) => cards,
        Err(err) => {
            if destination.created {
                cleanup_created_board(store, &destination.id);
            }
            return Err(internal(err));
        }
    };
    let mut lines = vec![format!(
        "created {} cards in board '{}' (board: {})",
        created_cards.len(),
        destination.slug,
        if destination.created {
            "created"
        } else {
            "existing"
        }
    )];
    lines.extend(
        created_cards
            .into_iter()
            .enumerate()
            .map(|(idx, card)| format!("{} {} {}", idx + 1, card.key, card.title)),
    );
    Ok(lines.join("\n"))
}

pub(crate) fn create_card_in_backlog(
    store: &mut Store,
    p: CreateBacklogCardParams,
) -> Result<String, ErrorData> {
    let backlog = store.ensure_default_board().map_err(internal)?;
    let card = store
        .create_card(
            &backlog.id,
            None,
            &p.title,
            p.body.as_deref().unwrap_or(""),
            "agent",
        )
        .map_err(internal)?;
    Ok(format!("created {} in Backlog (board: backlog)", card.key))
}
