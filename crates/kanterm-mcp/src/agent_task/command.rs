use anyhow::{anyhow, Context, Result};
use kanterm_core::AgentHandoff;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::targets::DeliveryTarget;

pub(super) struct VerificationProcess {
    pub(super) command: String,
    pub(super) passed: bool,
    pub(super) summary: String,
}

pub(super) fn run_target_command(
    target: &DeliveryTarget,
    handoff: &AgentHandoff,
    prompt: &str,
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
    command_target.configure_process(&mut command)?;
    let prompt_on_stdin = command_target.attach_prompt(&mut command, prompt);
    let mut child = command
        .env("KANTERM_HANDOFF_ID", &handoff.id)
        .env("KANTERM_HANDOFF_FROM_AGENT", &handoff.from_agent)
        .env("KANTERM_HANDOFF_TO_AGENT", &handoff.to_agent)
        .env("KANTERM_HANDOFF_SUBJECT", &handoff.subject)
        .stdin(if prompt_on_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning target command '{}'", command_target.command))?;
    if prompt_on_stdin {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("target command stdin was not available"))?;
        stdin.write_all(prompt.as_bytes())?;
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

pub(super) fn run_verification_command(
    target: &DeliveryTarget,
    verification_command: &str,
    verification_args: &[String],
) -> Result<VerificationProcess> {
    let DeliveryTarget::Command(command_target) = target else {
        return Err(anyhow!(
            "target '{}' is not a command target",
            target.name()
        ));
    };
    command_target.ensure_verification_supported()?;
    let mut command = Command::new(verification_command);
    command.args(verification_args);
    if let Some(repo) = &command_target.repo {
        command.current_dir(repo);
    }
    command_target.configure_process(&mut command)?;
    let output = command
        .output()
        .with_context(|| format!("spawning verification command '{verification_command}'"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = if !output.status.success() && !stderr.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    let summary = if detail.is_empty() {
        format!("exited with {}", output.status)
    } else {
        format!("exited with {}: {}", output.status, truncate(detail, 2_000))
    };
    Ok(VerificationProcess {
        command: std::iter::once(verification_command)
            .chain(verification_args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        passed: output.status.success(),
        summary,
    })
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::{CommandTarget, PromptTransport, TargetPolicy};

    #[test]
    fn argument_transport_delivers_the_exact_packet_to_a_headless_process() {
        let packet = "kanterm-agent-work-packet/v1\nprofile: execute\nquoted: $HOME `noop`\n";
        let target = DeliveryTarget::Command(CommandTarget {
            name: "cursor-mock".into(),
            agent: Some("cursor".into()),
            repo: None,
            command: "printf".into(),
            args: vec!["%s".into()],
            prompt_transport: PromptTransport::Argument,
            policy: TargetPolicy::default(),
        });
        let handoff = AgentHandoff {
            id: "handoff-1".into(),
            from_agent: "sender".into(),
            to_agent: "cursor".into(),
            board_id: None,
            card_key: None,
            subject: "test".into(),
            body: "body".into(),
            status: "claimed".into(),
            claimed_by: Some("cursor".into()),
            claimed_at: None,
            lease_expires_at: None,
            completed_at: None,
            failed_at: None,
            result_text: None,
            last_error: None,
            created_at: 0,
            updated_at: 0,
        };

        assert_eq!(
            run_target_command(&target, &handoff, packet).unwrap(),
            packet
        );
    }
}
