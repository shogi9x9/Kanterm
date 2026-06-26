use kanban_core::Store;
use rmcp::ErrorData;

use crate::error::{bad_param, internal};
use crate::params::UpdateParams;
use crate::workflow::RunWorkflowArgs;

pub(super) fn workflow_trigger(
    p: &UpdateParams,
    store: &Store,
    board_id: &str,
) -> Result<Option<RunWorkflowArgs>, ErrorData> {
    let any_workflow_field = p.workflow.is_some()
        || p.workflow_step.is_some()
        || p.workflow_targets.is_some()
        || p.workflow_from_agent.is_some();
    if !any_workflow_field {
        return Ok(None);
    }
    if p.complete_note.is_none() {
        return Err(bad_param(
            "workflow trigger fields require complete_note; workflows run only on card completion",
        ));
    }
    let workflow = p
        .workflow
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| bad_param("workflow is required when using workflow trigger fields"))?;
    let board = store.board_by_id_or_slug(board_id).map_err(internal)?.slug;
    Ok(Some(RunWorkflowArgs {
        workflow: workflow.into(),
        event: "complete".into(),
        step: p.workflow_step.clone(),
        from_agent: p
            .workflow_from_agent
            .clone()
            .or_else(|| p.claim.clone())
            .unwrap_or_else(|| "agent".into()),
        board: Some(board),
        card: Some(p.key.clone()),
        targets: p.workflow_targets.as_ref().map(Into::into),
    }))
}
