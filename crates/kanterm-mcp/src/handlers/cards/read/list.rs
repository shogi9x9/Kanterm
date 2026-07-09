mod filters;
mod format;

use kanterm_core::Store;
use rmcp::ErrorData;

use crate::error::internal;
use crate::lookup::{columns_by_id, resolve_board};
use crate::params::ListParams;

use self::filters::{filter_context, list_entry};
use self::format::format_entry;
use super::super::queue::QueueMode;

pub(crate) fn list_cards(
    store: &Store,
    default_board_id: &str,
    p: ListParams,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let names = columns_by_id(store, &board_id)?;
    let cards = match p.query.as_deref() {
        Some(query) if !query.trim().is_empty() => {
            store.search_cards(&board_id, query).map_err(internal)?
        }
        _ => store.cards(&board_id).map_err(internal)?,
    };
    let labels = store.labels_by_card(&board_id).map_err(internal)?;
    let queue = p.queue.as_deref().map(QueueMode::parse).transpose()?;
    let ranked = p.ranked.unwrap_or(false);
    let context = filter_context(&p, &names, queue);
    let mut entries = Vec::new();
    for c in &cards {
        let readiness = store.card_readiness(&board_id, &c.key).map_err(internal)?;
        let Some(entry) = list_entry(c, &readiness, &context) else {
            continue;
        };
        entries.push(format_entry(c, &readiness, &labels, entry, ranked));
    }
    if ranked {
        entries.sort_by_key(|(rank, key, _)| (*rank, key.clone()));
    }
    let lines = entries
        .into_iter()
        .map(|(_, _, line)| line)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        Ok("(no matching cards)".into())
    } else {
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanterm_core::CardPatch;

    #[test]
    fn list_cards_query_matches_next_action() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        store
            .create_card(&board.id, None, "ordinary title", "", "test")
            .unwrap();
        store
            .update_card(
                &board.id,
                "KB-1",
                &CardPatch {
                    next_action: Some("Run cargo mutants".to_string()),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        let out = list_cards(
            &store,
            &board.id,
            ListParams {
                board: None,
                column: None,
                agent_state: None,
                status: None,
                query: Some("mutants".to_string()),
                stale: None,
                agent_weight_max: None,
                agent_effort: None,
                suggested_model: None,
                expected_tokens_min: None,
                expected_tokens_max: None,
                human_intervention: None,
                queue: None,
                ranked: None,
            },
        )
        .unwrap();

        assert!(out.contains("KB-1"));
        assert!(out.contains("[next]"));

        let fresh = list_cards(
            &store,
            &board.id,
            ListParams {
                board: None,
                column: None,
                agent_state: None,
                status: None,
                query: None,
                stale: Some(false),
                agent_weight_max: None,
                agent_effort: None,
                suggested_model: None,
                expected_tokens_min: None,
                expected_tokens_max: None,
                human_intervention: None,
                queue: None,
                ranked: None,
            },
        )
        .unwrap();
        assert!(fresh.contains("KB-1"));

        let stale = list_cards(
            &store,
            &board.id,
            ListParams {
                board: None,
                column: None,
                agent_state: None,
                status: None,
                query: None,
                stale: Some(true),
                agent_weight_max: None,
                agent_effort: None,
                suggested_model: None,
                expected_tokens_min: None,
                expected_tokens_max: None,
                human_intervention: None,
                queue: None,
                ranked: None,
            },
        )
        .unwrap();
        assert!(stale.contains("no matching"));
    }
}
