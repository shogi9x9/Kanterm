use anyhow::{anyhow, Context, Result};
use kanterm_core::{AgentHandoff, AgentWorkPacket};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::targets::{DeliveryTarget, PromptTransport, TargetPolicy};

#[derive(Debug, Clone)]
pub(crate) struct BridgeCommand {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) prompt: BridgePrompt,
    pub(crate) prompt_transport: PromptTransport,
    pub(crate) policy: Option<TargetPolicy>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BridgePrompt {
    Body,
    Packet,
}

#[derive(Debug, Clone)]
pub(crate) enum Delivery {
    Bridge(BridgeCommand),
    Command(BridgeCommand),
}

pub(super) enum DeliveryOutcome {
    Delivered,
    Completed(String),
}

pub(super) fn deliver(
    handoff: &AgentHandoff,
    bridge: Option<&Delivery>,
) -> Result<DeliveryOutcome> {
    match bridge {
        Some(Delivery::Bridge(command)) => {
            run_bridge(handoff, command, false).map(|_| DeliveryOutcome::Delivered)
        }
        Some(Delivery::Command(command)) => {
            run_bridge(handoff, command, true).map(DeliveryOutcome::Completed)
        }
        None => {
            println!("{}", serde_json::to_string(&handoff_payload(handoff))?);
            Ok(DeliveryOutcome::Delivered)
        }
    }
}

pub(super) fn delivery_from_target(target: &DeliveryTarget) -> Result<Delivery> {
    match target {
        DeliveryTarget::Command(command) => Ok(Delivery::Command(BridgeCommand {
            program: command.command.clone(),
            args: command.args.clone(),
            cwd: command.repo.clone(),
            prompt: BridgePrompt::Packet,
            prompt_transport: command.prompt_transport,
            policy: Some(command.policy.clone()),
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

fn run_bridge(
    handoff: &AgentHandoff,
    bridge: &BridgeCommand,
    capture_stdout: bool,
) -> Result<String> {
    let mut command = Command::new(&bridge.program);
    command.args(&bridge.args);
    if let Some(cwd) = &bridge.cwd {
        command.current_dir(cwd);
    }
    if let Some(policy) = &bridge.policy {
        policy.configure_process(&mut command)?;
    }
    let rendered_prompt = bridge_prompt(handoff, bridge.prompt)?;
    let prompt_on_stdin = match bridge.prompt_transport {
        PromptTransport::Stdin => true,
        PromptTransport::Argument => {
            command.arg(&rendered_prompt);
            false
        }
    };
    command.env(
        "KANTERM_DELIVERY_MODE",
        match bridge.prompt {
            BridgePrompt::Body => "handoff-body",
            BridgePrompt::Packet => "packet",
        },
    );
    let command = command
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
        .stdin(if prompt_on_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    command.stdout(if capture_stdout {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    let mut child = command
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawning bridge command '{}'", bridge.program))?;
    if prompt_on_stdin {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("bridge command stdin was not available"))?;
        stdin.write_all(rendered_prompt.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow!("bridge command exited with {}", output.status))
    }
}

fn bridge_prompt(handoff: &AgentHandoff, prompt: BridgePrompt) -> Result<String> {
    match prompt {
        BridgePrompt::Body => Ok(handoff.body.clone()),
        BridgePrompt::Packet => AgentWorkPacket::execute_handoff(handoff).render(),
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
