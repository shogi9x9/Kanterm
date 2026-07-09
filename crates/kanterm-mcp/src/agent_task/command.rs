use anyhow::{anyhow, Context, Result};
use kanterm_core::AgentHandoff;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::targets::DeliveryTarget;

pub(super) fn run_target_command(
    target: &DeliveryTarget,
    handoff: &AgentHandoff,
) -> Result<String> {
    let DeliveryTarget::Command(command_target) = target else {
        return Err(anyhow!(
            "target '{}' is not a command target",
            target.name()
        ));
    };
    let mut command = Command::new(&command_target.command);
    command.args(&command_target.args);
    if let Some(repo) = &command_target.repo {
        command.current_dir(repo);
    }
    let mut child = command
        .env("KANTERM_HANDOFF_ID", &handoff.id)
        .env("KANTERM_HANDOFF_FROM_AGENT", &handoff.from_agent)
        .env("KANTERM_HANDOFF_TO_AGENT", &handoff.to_agent)
        .env("KANTERM_HANDOFF_SUBJECT", &handoff.subject)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning target command '{}'", command_target.command))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(agent_prompt(handoff).as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "target command exited with {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn agent_prompt(handoff: &AgentHandoff) -> String {
    format!(
        "Kanterm handoff received.\n\nhandoff_id: {}\nfrom_agent: {}\nto_agent: {}\nsubject: {}\ncard_key: {}\n\nTask:\n{}\n",
        handoff.id,
        handoff.from_agent,
        handoff.to_agent,
        handoff.subject,
        handoff.card_key.as_deref().unwrap_or(""),
        handoff.body
    )
}
