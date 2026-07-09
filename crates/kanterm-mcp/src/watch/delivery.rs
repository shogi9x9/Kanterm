use anyhow::{anyhow, Context, Result};
use kanterm_core::AgentHandoff;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::targets::DeliveryTarget;

#[derive(Debug, Clone)]
pub(crate) struct BridgeCommand {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) prompt: BridgePrompt,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BridgePrompt {
    Body,
    Formatted,
}

#[derive(Debug, Clone)]
pub(crate) enum Delivery {
    Command(BridgeCommand),
}

pub(super) fn deliver(handoff: &AgentHandoff, bridge: Option<&Delivery>) -> Result<()> {
    match bridge {
        Some(Delivery::Command(command)) => run_bridge(handoff, command),
        None => {
            println!("{}", serde_json::to_string(&handoff_payload(handoff))?);
            Ok(())
        }
    }
}

pub(super) fn delivery_from_target(target: &DeliveryTarget) -> Result<Delivery> {
    match target {
        DeliveryTarget::Command(command) => Ok(Delivery::Command(BridgeCommand {
            program: command.command.clone(),
            args: command.args.clone(),
            cwd: command.repo.clone(),
            prompt: BridgePrompt::Formatted,
        })),
        DeliveryTarget::Interactive(target) => Err(anyhow!(
            "interactive target '{}' is configured for adapter '{}' session '{}' pane '{}' but watcher delivery is not implemented yet",
            target.name,
            target.adapter.as_deref().unwrap_or("unknown"),
            target.session.as_deref().unwrap_or(""),
            target.pane.as_deref().unwrap_or("")
        )),
    }
}

fn run_bridge(handoff: &AgentHandoff, bridge: &BridgeCommand) -> Result<()> {
    let mut command = Command::new(&bridge.program);
    command.args(&bridge.args);
    if let Some(cwd) = &bridge.cwd {
        command.current_dir(cwd);
    }
    let mut child = command
        .env("KANTERM_HANDOFF_ID", &handoff.id)
        .env("KANTERM_HANDOFF_FROM_AGENT", &handoff.from_agent)
        .env("KANTERM_HANDOFF_TO_AGENT", &handoff.to_agent)
        .env("KANTERM_HANDOFF_SUBJECT", &handoff.subject)
        .env("KANTERM_HANDOFF_STATUS", &handoff.status)
        .env("KANTERM_HANDOFF_CLAIMED_BY", opt(&handoff.claimed_by))
        .env("KANTERM_HANDOFF_BOARD_ID", opt(&handoff.board_id))
        .env("KANTERM_HANDOFF_CARD_KEY", opt(&handoff.card_key))
        .env(
            "KANTERM_HANDOFF_LEASE_EXPIRES_AT",
            handoff
                .lease_expires_at
                .map(|v| v.to_string())
                .unwrap_or_default(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawning bridge command '{}'", bridge.program))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(bridge_prompt(handoff, bridge.prompt).as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("bridge command exited with {status}"))
    }
}

fn bridge_prompt(handoff: &AgentHandoff, prompt: BridgePrompt) -> String {
    match prompt {
        BridgePrompt::Body => handoff.body.clone(),
        BridgePrompt::Formatted => format!(
            "Kanterm handoff received.\n\nhandoff_id: {}\nfrom_agent: {}\nto_agent: {}\nsubject: {}\nboard_id: {}\ncard_key: {}\nlease_expires_at: {}\n\nTask:\n{}\n",
            handoff.id,
            handoff.from_agent,
            handoff.to_agent,
            handoff.subject,
            opt(&handoff.board_id),
            opt(&handoff.card_key),
            handoff.lease_expires_at.map(|v| v.to_string()).unwrap_or_default(),
            handoff.body
        ),
    }
}

fn handoff_payload(handoff: &AgentHandoff) -> serde_json::Value {
    serde_json::json!({
        "id": handoff.id,
        "from_agent": handoff.from_agent,
        "to_agent": handoff.to_agent,
        "board_id": handoff.board_id,
        "card_key": handoff.card_key,
        "subject": handoff.subject,
        "body": handoff.body,
        "status": handoff.status,
        "claimed_by": handoff.claimed_by,
        "lease_expires_at": handoff.lease_expires_at,
        "created_at": handoff.created_at,
        "updated_at": handoff.updated_at,
    })
}

fn opt(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("")
}
