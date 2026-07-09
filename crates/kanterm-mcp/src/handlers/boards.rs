use kanterm_core::{BoardColumnTemplate, Store, PROTECTED_BOARD_SLUG};
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::params::ManageBoardsParams;

pub(crate) fn manage_boards(store: &mut Store, p: ManageBoardsParams) -> Result<String, ErrorData> {
    let resolve = |store: &Store, slug: &str| -> Result<String, ErrorData> {
        store
            .board_by_slug(slug)
            .map_err(internal)?
            .map(|b| b.id)
            .ok_or_else(|| bad_param(format!("no board '{slug}'")))
    };
    match p.action.as_str() {
        "create" => {
            let name = p
                .name
                .ok_or_else(|| bad_param("`name` is required for create"))?;
            let template = match p.template {
                Some(template_key) => BoardColumnTemplate::from_key(&template_key)
                    .ok_or_else(|| bad_param("`template` must be planning, workflow, or simple"))?,
                None => BoardColumnTemplate::DEFAULT_PROJECT,
            };
            let mut board = store.create_board(&name, template).map_err(internal)?;
            if let Some(context) = p.agent_context.as_deref() {
                board = store
                    .update_board_agent_context(&board.id, Some(context))
                    .map_err(internal)?;
            }
            Ok(format!(
                "created board '{}' (slug: {}, template: {}){}",
                board.name,
                board.slug,
                template.key(),
                if board.agent_context.is_some() {
                    " with agent_context"
                } else {
                    ""
                }
            ))
        }
        "archive" => {
            let slug = p
                .board
                .ok_or_else(|| bad_param("`board` slug is required for archive"))?;
            let id = resolve(store, &slug)?;
            store.archive_board(&id).map_err(internal)?;
            Ok(format!(
                "archived board '{slug}' (hidden, cards kept; unarchive to restore)"
            ))
        }
        "unarchive" => {
            let slug = p
                .board
                .ok_or_else(|| bad_param("`board` slug is required for unarchive"))?;
            let id = resolve(store, &slug)?;
            store.unarchive_board(&id).map_err(internal)?;
            Ok(format!("unarchived board '{slug}'"))
        }
        "delete" => {
            let slug = p
                .board
                .ok_or_else(|| bad_param("`board` slug is required for delete"))?;
            if slug == PROTECTED_BOARD_SLUG {
                return Err(bad_param("the Backlog board cannot be deleted"));
            }
            let id = resolve(store, &slug)?;
            store.delete_board(&id).map_err(internal)?;
            Ok(format!("deleted board '{slug}'"))
        }
        "reorder" => {
            let slug = p
                .board
                .ok_or_else(|| bad_param("`board` slug is required for reorder"))?;
            let dir = match p.direction.as_deref() {
                Some("up") => -1,
                Some("down") => 1,
                _ => return Err(bad_param("`direction` must be \"up\" or \"down\"")),
            };
            let id = resolve(store, &slug)?;
            store.reorder_board(&id, dir).map_err(internal)?;
            Ok(format!(
                "moved board '{slug}' {}",
                p.direction.unwrap_or_default()
            ))
        }
        "set_context" => {
            let slug = p
                .board
                .ok_or_else(|| bad_param("`board` slug is required for set_context"))?;
            let context = p
                .agent_context
                .ok_or_else(|| bad_param("`agent_context` is required for set_context"))?;
            let id = resolve(store, &slug)?;
            let board = store
                .update_board_agent_context(&id, Some(&context))
                .map_err(internal)?;
            Ok(format!("updated board '{}' agent_context", board.slug))
        }
        "clear_context" => {
            let slug = p
                .board
                .ok_or_else(|| bad_param("`board` slug is required for clear_context"))?;
            let id = resolve(store, &slug)?;
            let board = store.update_board_agent_context(&id, None).map_err(internal)?;
            Ok(format!("cleared board '{}' agent_context", board.slug))
        }
        other => Err(bad_param(format!(
            "unknown action '{other}'; use create|archive|unarchive|delete|reorder|set_context|clear_context"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_without_template_uses_workflow_default() {
        let mut store = Store::open_in_memory().unwrap();
        store.ensure_default_board().unwrap();

        let msg = manage_boards(
            &mut store,
            ManageBoardsParams {
                action: "create".to_string(),
                name: Some("Work".to_string()),
                template: None,
                board: None,
                direction: None,
                agent_context: None,
            },
        )
        .unwrap();

        assert!(msg.contains("template: workflow"));
        let board = store.board_by_slug("work").unwrap().unwrap();
        let columns: Vec<String> = store
            .columns(&board.id)
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(
            columns,
            BoardColumnTemplate::Workflow
                .columns()
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn delete_rejects_backlog_before_core_call() {
        let mut store = Store::open_in_memory().unwrap();
        store.ensure_default_board().unwrap();

        let err = manage_boards(
            &mut store,
            ManageBoardsParams {
                action: "delete".to_string(),
                name: None,
                template: None,
                board: Some("backlog".to_string()),
                direction: None,
                agent_context: None,
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("Backlog board cannot be deleted"));
    }
}
