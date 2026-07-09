use kanterm_core::{Card, Store};
use rmcp::ErrorData;

use crate::error::internal;
use crate::lookup::resolve_board;
use crate::params::BoardParam;
use crate::render::{board_execution_suffix, due_suffix, label_suffix};

pub(crate) fn get_board(
    store: &Store,
    default_board_id: &str,
    p: BoardParam,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    let board = store.board_by_id_or_slug(&board_id).map_err(internal)?;
    let cols = store.columns(&board_id).map_err(internal)?;
    let cards = store.cards(&board_id).map_err(internal)?;
    let labels = store.labels_by_card(&board_id).map_err(internal)?;
    let mut out = String::new();
    if let Some(context) = board.agent_context.as_deref() {
        out.push_str("board_agent_context:\n");
        out.push_str(context);
        out.push_str("\n\n");
    }
    for col in &cols {
        let in_col: Vec<&Card> = cards.iter().filter(|c| c.column_id == col.id).collect();
        out.push_str(&format!("## {} ({})\n", col.name, in_col.len()));
        for c in in_col {
            out.push_str(&format!(
                "- {} {}{}{}{}\n",
                c.key,
                c.title,
                due_suffix(c),
                label_suffix(&labels, &c.id),
                board_execution_suffix(c)
            ));
        }
        out.push('\n');
    }

    let boards = store.list_boards_all().map_err(internal)?;
    let line = |archived: bool| {
        boards
            .iter()
            .filter(|b| b.archived_at.is_some() == archived)
            .map(|b| {
                let context = if b.agent_context.is_some() {
                    " [context]"
                } else {
                    ""
                };
                if b.id == board_id {
                    format!("{}{} (current)", b.slug, context)
                } else {
                    format!("{}{}", b.slug, context)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str("---\nboards: ");
    out.push_str(&line(false));
    let archived = line(true);
    if !archived.is_empty() {
        out.push_str(&format!("\narchived boards: {archived}"));
    }
    Ok(out)
}
