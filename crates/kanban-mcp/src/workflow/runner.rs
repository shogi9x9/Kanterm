use anyhow::{anyhow, Context, Result};
use kanban_core::{HandoffDraft, Store};
use std::fs;

use super::args::RunWorkflowArgs;
use super::parser::{parse_workflow, SendHandoffRule, Workflow, WorkflowAction, WorkflowStep};
use super::template::{default_body_template, render_template};
use crate::targets::{DeliveryTarget, TargetConfig};

pub(crate) fn run(store: &mut Store, args: RunWorkflowArgs) -> Result<()> {
    println!("{}", run_summary(store, args)?);
    Ok(())
}

pub(crate) fn run_summary(store: &mut Store, args: RunWorkflowArgs) -> Result<String> {
    let source = fs::read_to_string(&args.workflow)
        .with_context(|| format!("reading workflow {}", args.workflow.display()))?;
    let workflow = parse_workflow(&source)?;
    if args.event != "complete" {
        return Err(anyhow!("unsupported workflow event: {}", args.event));
    }
    let step = selected_step(&workflow, args.step.as_deref())?;
    let Some(WorkflowAction::SendHandoff(rule)) = step.on_complete.as_ref() else {
        return Ok(format!(
            "workflow: {}\nstep: {}\naction: none",
            workflow.name, step.name
        ));
    };
    let target_config = args
        .targets
        .as_deref()
        .map(TargetConfig::load)
        .transpose()?;
    let target = resolve_target(rule, target_config.as_ref())?;
    let to_agent = resolve_to_agent(rule, target)?;
    let repo = resolve_repo(rule, target);
    let board_id = args
        .board
        .as_deref()
        .map(|slug| {
            store
                .board_by_slug(slug)?
                .map(|board| board.id)
                .ok_or_else(|| anyhow!("no board '{slug}'"))
        })
        .transpose()?;
    let body = render_template(
        rule.body.as_deref().unwrap_or(default_body_template()),
        &workflow,
        step,
        rule,
        &args,
        to_agent.as_str(),
        repo.as_deref(),
    );
    let subject = render_template(
        &rule.subject,
        &workflow,
        step,
        rule,
        &args,
        to_agent.as_str(),
        repo.as_deref(),
    );
    let handoff = store.create_handoff(&HandoffDraft {
        from_agent: args.from_agent,
        to_agent,
        board_id,
        card_key: args.card,
        subject,
        body,
    })?;
    Ok(format!(
        "workflow: {}\nstep: {}\naction: send_handoff\nhandoff_id: {}\nto_agent: {}",
        workflow.name, step.name, handoff.id, handoff.to_agent
    ))
}

fn selected_step<'a>(
    workflow: &'a Workflow,
    explicit_step: Option<&str>,
) -> Result<&'a WorkflowStep> {
    let step_name = explicit_step
        .or(workflow.initial_step.as_deref())
        .ok_or_else(|| anyhow!("--step is required when workflow has no initial_step"))?;
    workflow
        .steps
        .iter()
        .find(|step| step.name == step_name)
        .ok_or_else(|| anyhow!("workflow step '{step_name}' was not found"))
}

fn resolve_target<'a>(
    rule: &SendHandoffRule,
    config: Option<&'a TargetConfig>,
) -> Result<Option<&'a DeliveryTarget>> {
    rule.target
        .as_deref()
        .map(|name| {
            config
                .ok_or_else(|| anyhow!("--targets is required when workflow uses target '{name}'"))?
                .find(name)
        })
        .transpose()
}

fn resolve_to_agent(rule: &SendHandoffRule, target: Option<&DeliveryTarget>) -> Result<String> {
    if let Some(to_agent) = rule
        .to_agent
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(to_agent.to_string());
    }
    if let Some(agent) = target.and_then(DeliveryTarget::agent) {
        return Ok(agent.to_string());
    }
    if let Some(name) = rule
        .target
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(name.to_string());
    }
    Err(anyhow!("send_handoff target could not resolve a recipient"))
}

fn resolve_repo(rule: &SendHandoffRule, target: Option<&DeliveryTarget>) -> Option<String> {
    rule.repo.clone().or_else(|| {
        target
            .and_then(DeliveryTarget::repo)
            .map(|path| path.display().to_string())
    })
}
