use anyhow::Result;
use kanterm_core::{AgentHandoff, HandoffStatusPatch, Store};

use super::args::AgentTaskArgs;
use super::command::run_target_command;
use super::completion::complete_card;
use crate::targets::TargetConfig;
use crate::workflow::{self, RunWorkflowArgs};

pub(crate) fn run(store: &mut Store, args: AgentTaskArgs) -> Result<()> {
    println!("{}", run_summary(store, args)?);
    Ok(())
}

fn run_summary(store: &mut Store, args: AgentTaskArgs) -> Result<String> {
    let Some(handoff) = store
        .list_handoffs(Some(&args.for_agent), false, 1)?
        .into_iter()
        .next()
    else {
        return Ok(format!("agent_task: none\nfor_agent: {}", args.for_agent));
    };
    let claimed = store.claim_handoff(
        &handoff.id,
        &args.for_agent,
        Some(&args.claim_token),
        args.lease_minutes,
    )?;
    match run_claimed_task(store, &args, &claimed) {
        Ok(summary) => {
            store.update_handoff_status(
                &claimed.id,
                &args.for_agent,
                Some(&args.claim_token),
                &HandoffStatusPatch {
                    status: "completed".into(),
                    note: Some("completed by run-agent-task".into()),
                },
            )?;
            Ok(summary)
        }
        Err(err) => {
            let _ = store.update_handoff_status(
                &claimed.id,
                &args.for_agent,
                Some(&args.claim_token),
                &HandoffStatusPatch {
                    status: "failed".into(),
                    note: Some(err.to_string()),
                },
            );
            Err(err)
        }
    }
}

fn run_claimed_task(
    store: &mut Store,
    args: &AgentTaskArgs,
    handoff: &AgentHandoff,
) -> Result<String> {
    let config = TargetConfig::load(&args.targets)?;
    let target = config.find(&args.target)?;
    let output = run_target_command(target, handoff)?;
    let agent = args
        .from_agent
        .clone()
        .unwrap_or_else(|| args.for_agent.clone());
    complete_card(store, args, &agent, &output)?;
    let mut summary = format!(
        "agent_task: completed\nhandoff_id: {}\ncard: {}\nagent_output:\n{}",
        handoff.id,
        args.card,
        output.trim()
    );
    if let Some(workflow) = &args.workflow {
        let workflow_summary = workflow::run_summary(
            store,
            RunWorkflowArgs {
                workflow: workflow.clone(),
                event: "complete".into(),
                step: args.workflow_step.clone(),
                from_agent: agent,
                board: Some(args.board.clone()),
                card: Some(args.card.clone()),
                targets: args.workflow_targets.clone(),
            },
        )?;
        summary.push_str("\nworkflow_triggered:\n");
        summary.push_str(&workflow_summary);
    }
    Ok(summary)
}
