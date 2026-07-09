use kanterm_core::Store;
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::lookup::{resolve_board, resolve_column};
use crate::params::ManageColumnsParams;

pub(crate) fn manage_columns(
    store: &mut Store,
    default_board_id: &str,
    p: ManageColumnsParams,
) -> Result<String, ErrorData> {
    let board_id = resolve_board(store, default_board_id, p.board.as_deref())?;
    match p.action.as_str() {
        "add" => {
            let name = p
                .name
                .ok_or_else(|| bad_param("`name` is required for add"))?;
            let c = store.add_column(&board_id, &name).map_err(internal)?;
            Ok(format!("added column {}", c.name))
        }
        "rename" => {
            let target = p
                .column
                .ok_or_else(|| bad_param("`column` is required for rename"))?;
            let new = p
                .new_name
                .ok_or_else(|| bad_param("`new_name` is required for rename"))?;
            let id = resolve_column(store, &board_id, &target)?;
            store.rename_column(&id, &new).map_err(internal)?;
            Ok(format!("renamed '{target}' -> '{new}'"))
        }
        "delete" => {
            let target = p
                .column
                .ok_or_else(|| bad_param("`column` is required for delete"))?;
            let to =
                p.to.ok_or_else(|| bad_param("`to` (destination column) is required for delete"))?;
            let victim = resolve_column(store, &board_id, &target)?;
            let dest = resolve_column(store, &board_id, &to)?;
            store
                .delete_column(&board_id, &victim, &dest)
                .map_err(internal)?;
            Ok(format!("deleted '{target}'; its cards moved to '{to}'"))
        }
        "reorder" => {
            let target = p
                .column
                .ok_or_else(|| bad_param("`column` is required for reorder"))?;
            let dir = match p.direction.as_deref() {
                Some("left") => -1,
                Some("right") => 1,
                _ => return Err(bad_param("`direction` must be \"left\" or \"right\"")),
            };
            let id = resolve_column(store, &board_id, &target)?;
            store
                .reorder_column(&board_id, &id, dir)
                .map_err(internal)?;
            Ok(format!(
                "moved '{target}' {}",
                p.direction.unwrap_or_default()
            ))
        }
        other => Err(bad_param(format!(
            "unknown action '{other}'; use add|rename|delete|reorder"
        ))),
    }
}
