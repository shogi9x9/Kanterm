use std::fmt;

use anyhow::Result;
use kanterm_core::{
    AgentHandoff, AgentWorkPacket, HandoffListQuery, HandoffStatusPatch, Store,
    AGENT_WORK_PACKET_VERSION,
};

use super::args::AgentTaskArgs;
use super::command::{run_target_command, run_verification_command};
use super::completion::apply_completion_policy;
use crate::targets::TargetConfig;
use crate::workflow::{self, RunWorkflowArgs};

pub(crate) fn run(store: &mut Store, args: AgentTaskArgs) -> Result<()> {
    println!("{}", run_summary(store, args)?);
    Ok(())
}

fn run_summary(store: &mut Store, args: AgentTaskArgs) -> Result<String> {
    let Some(handoff) = store
        .list_handoffs(HandoffListQuery {
            recipient: Some(&args.for_agent),
            claimable_only: true,
            limit: 1,
            ..Default::default()
        })?
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
        Ok(completed) => {
            store.update_handoff_status(
                &claimed.id,
                &args.for_agent,
                Some(&args.claim_token),
                &HandoffStatusPatch {
                    status: "completed".into(),
                    note: Some(completed.agent_output),
                },
            )?;
            Ok(completed.summary)
        }
        Err(err) => {
            settle_claimed_error(store, &args, &claimed, &err)?;
            Err(err)
        }
    }
}

fn run_claimed_task(
    store: &mut Store,
    args: &AgentTaskArgs,
    handoff: &AgentHandoff,
) -> Result<ClaimedTaskResult> {
    let config = TargetConfig::load(&args.targets)?;
    let target = config.find(&args.target)?;
    let agent = args
        .from_agent
        .clone()
        .unwrap_or_else(|| args.for_agent.clone());
    let prepared_workflow = args
        .workflow
        .as_ref()
        .map(|workflow| {
            workflow::prepare(
                store,
                RunWorkflowArgs {
                    workflow: workflow.clone(),
                    event: "complete".into(),
                    step: args.workflow_step.clone(),
                    from_agent: agent.clone(),
                    board: Some(args.board.clone()),
                    card: Some(args.card.clone()),
                    targets: args.workflow_targets.clone(),
                },
            )
        })
        .transpose()?;
    let snapshot = store.execution_prompt_snapshot(&args.board, &args.card)?;
    let resume_delta =
        store.agent_work_packet_resume_delta(&handoff.id, &args.board, &args.card)?;
    let packet = if resume_delta.original_packet_sha256.is_some() {
        AgentWorkPacket::resume_with_delta(&snapshot, &resume_delta, &handoff.body)
    } else {
        AgentWorkPacket::execute_with_delivery_context(&snapshot, &handoff.body)
    };
    let profile = packet.profile().as_str();
    let prompt = packet.render()?;
    let attempt = store.start_agent_task_attempt(
        &handoff.id,
        target.name(),
        AGENT_WORK_PACKET_VERSION,
        profile,
        &prompt,
    )?;
    let output = match run_target_command(target, handoff, &prompt) {
        Ok(output) => {
            store.finish_agent_task_attempt(&attempt.id, "agent_succeeded", Some(output.trim()))?;
            output
        }
        Err(error) => {
            let note = error.to_string();
            store.finish_agent_task_attempt(&attempt.id, "agent_failed", Some(&note))?;
            return Err(error);
        }
    };
    let verification = args
        .verification_command
        .as_deref()
        .map(|command| run_verification_command(target, command, &args.verification_args))
        .transpose()?;
    let completion = apply_completion_policy(store, args, &agent, &output, verification.as_ref())?;
    let mut summary = format!(
        "agent_task: {}\nhandoff_id: {}\ncard: {}\nverification: {}\nagent_output:\n{}",
        completion.state,
        handoff.id,
        args.card,
        completion.verification_summary,
        output.trim()
    );
    if !completion.completed {
        return Err(anyhow::anyhow!(summary));
    }
    if let Some(workflow) = prepared_workflow {
        let workflow_summary = workflow::run_prepared(store, workflow).map_err(|error| {
            anyhow::Error::new(PostCompletionWorkflowError {
                message: format!(
                    "agent task completed and the card was archived, but workflow dispatch failed: {error}; run the workflow separately after correcting the failure"
                ),
            })
        })?;
        summary.push_str("\nworkflow_triggered:\n");
        summary.push_str(&workflow_summary);
    }
    Ok(ClaimedTaskResult {
        summary,
        agent_output: output.trim().to_string(),
    })
}

struct ClaimedTaskResult {
    summary: String,
    agent_output: String,
}

#[derive(Debug)]
struct PostCompletionWorkflowError {
    message: String,
}

impl fmt::Display for PostCompletionWorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PostCompletionWorkflowError {}

fn settle_claimed_error(
    store: &mut Store,
    args: &AgentTaskArgs,
    handoff: &AgentHandoff,
    error: &anyhow::Error,
) -> Result<()> {
    if error
        .downcast_ref::<PostCompletionWorkflowError>()
        .is_some()
    {
        store.update_handoff_status(
            &handoff.id,
            &args.for_agent,
            Some(&args.claim_token),
            &HandoffStatusPatch {
                status: "failed".into(),
                note: Some(error.to_string()),
            },
        )?;
    } else {
        store.requeue_handoff(
            &handoff.id,
            &args.for_agent,
            Some(&args.claim_token),
            Some(&error.to_string()),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kanterm_core::HandoffDraft;

    use super::*;

    fn claimed_handoff() -> (Store, AgentTaskArgs, AgentHandoff) {
        let mut store = Store::open_in_memory().unwrap();
        let registered = store.register_agent("worker", None, None, None).unwrap();
        let identity = registered.registration.assigned_identity;
        let token = registered.claim_token;
        let handoff = store
            .create_handoff(&HandoffDraft {
                from_agent: "sender".into(),
                to_agent: identity.clone(),
                board_id: None,
                card_key: None,
                subject: "work".into(),
                body: "do work".into(),
            })
            .unwrap();
        let claimed = store
            .claim_handoff(&handoff.id, &identity, Some(&token), None)
            .unwrap();
        let args = AgentTaskArgs {
            for_agent: identity,
            claim_token: token,
            targets: PathBuf::from("targets.yaml"),
            target: "target".into(),
            board: "board".into(),
            card: "CARD-1".into(),
            lease_minutes: None,
            complete_note: None,
            verification_command: None,
            verification_args: Vec::new(),
            workflow: None,
            workflow_step: None,
            workflow_targets: None,
            from_agent: None,
        };
        (store, args, claimed)
    }

    #[test]
    fn post_completion_workflow_failure_is_terminal() {
        let (mut store, args, handoff) = claimed_handoff();
        let error = anyhow::Error::new(PostCompletionWorkflowError {
            message: "workflow dispatch failed".into(),
        });

        settle_claimed_error(&mut store, &args, &handoff, &error).unwrap();

        let updated = store.handoff_by_id(&handoff.id).unwrap().unwrap();
        assert_eq!(updated.status, "failed");
        assert_eq!(
            updated.last_error.as_deref(),
            Some("workflow dispatch failed")
        );
    }

    #[test]
    fn pre_completion_failure_remains_resumable() {
        let (mut store, args, handoff) = claimed_handoff();
        let error = anyhow::anyhow!("verification failed");

        settle_claimed_error(&mut store, &args, &handoff, &error).unwrap();

        let updated = store.handoff_by_id(&handoff.id).unwrap().unwrap();
        assert_eq!(updated.status, "pending");
        assert_eq!(updated.last_error.as_deref(), Some("verification failed"));
    }
}
