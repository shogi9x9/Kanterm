mod parser;

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use self::parser::parse_targets;

#[derive(Debug, Clone)]
pub(crate) enum DeliveryTarget {
    Command(CommandTarget),
    Interactive(InteractiveTarget),
}

#[derive(Debug, Clone)]
pub(crate) struct CommandTarget {
    pub(crate) name: String,
    pub(crate) agent: Option<String>,
    pub(crate) repo: Option<PathBuf>,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) prompt_transport: PromptTransport,
    pub(crate) policy: TargetPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptTransport {
    Stdin,
    Argument,
}

#[derive(Debug, Clone)]
pub(crate) struct InteractiveTarget {
    pub(crate) name: String,
    pub(crate) agent: Option<String>,
    pub(crate) adapter: Option<String>,
    pub(crate) session: Option<String>,
    pub(crate) pane: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetPolicy {
    pub(crate) delivery: String,
    pub(crate) environment: String,
    pub(crate) network: String,
    pub(crate) workspace: String,
    pub(crate) approval: String,
    pub(crate) verification: String,
    pub(crate) writable_paths: Vec<PathBuf>,
}

impl Default for TargetPolicy {
    fn default() -> Self {
        Self {
            delivery: "packet".into(),
            environment: "inherit".into(),
            network: "inherit".into(),
            workspace: "repo-write".into(),
            approval: "external".into(),
            verification: "command".into(),
            writable_paths: Vec::new(),
        }
    }
}

impl CommandTarget {
    pub(crate) fn configure_process(&self, command: &mut Command) -> Result<()> {
        self.policy.configure_process(command)
    }

    pub(crate) fn ensure_verification_supported(&self) -> Result<()> {
        if self.policy.verification == "command" {
            Ok(())
        } else {
            Err(anyhow!(
                "target '{}' declares verification: none and cannot run --verify-command",
                self.name
            ))
        }
    }

    pub(crate) fn attach_prompt(&self, command: &mut Command, prompt: &str) -> bool {
        match self.prompt_transport {
            PromptTransport::Stdin => true,
            PromptTransport::Argument => {
                command.arg(prompt);
                false
            }
        }
    }
}

impl TargetPolicy {
    pub(crate) fn configure_process(&self, command: &mut Command) -> Result<()> {
        if self.environment == "clean" {
            let path = std::env::var_os("PATH");
            command.env_clear();
            if let Some(path) = path {
                command.env("PATH", path);
            }
        }
        command
            .env("KANTERM_DELIVERY_MODE", &self.delivery)
            .env("KANTERM_NETWORK_POLICY", &self.network)
            .env("KANTERM_WORKSPACE_POLICY", &self.workspace)
            .env("KANTERM_APPROVAL_POLICY", &self.approval);
        if !self.writable_paths.is_empty() {
            command.env(
                "KANTERM_WRITABLE_PATHS",
                std::env::join_paths(&self.writable_paths)
                    .context("joining target writable_paths")?,
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TargetConfig {
    pub(super) targets: Vec<DeliveryTarget>,
}

impl TargetConfig {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("reading target config {}", path.display()))?;
        parse_targets(&source)
    }

    pub(crate) fn find(&self, name: &str) -> Result<&DeliveryTarget> {
        self.targets
            .iter()
            .find(|target| target.name() == name)
            .ok_or_else(|| anyhow!("target '{name}' was not found"))
    }
}

impl DeliveryTarget {
    pub(crate) fn name(&self) -> &str {
        match self {
            DeliveryTarget::Command(target) => &target.name,
            DeliveryTarget::Interactive(target) => &target.name,
        }
    }

    pub(crate) fn agent(&self) -> Option<&str> {
        match self {
            DeliveryTarget::Command(target) => target.agent.as_deref(),
            DeliveryTarget::Interactive(target) => target.agent.as_deref(),
        }
    }

    pub(crate) fn repo(&self) -> Option<&Path> {
        match self {
            DeliveryTarget::Command(target) => target.repo.as_deref(),
            DeliveryTarget::Interactive(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_policy_is_applied_as_machine_readable_process_environment() {
        let target = CommandTarget {
            name: "isolated".into(),
            agent: None,
            repo: Some(PathBuf::from("/workspace/project")),
            command: "agent-cli".into(),
            args: Vec::new(),
            prompt_transport: PromptTransport::Stdin,
            policy: TargetPolicy {
                delivery: "packet".into(),
                environment: "clean".into(),
                network: "inherit".into(),
                workspace: "repo-write".into(),
                approval: "never".into(),
                verification: "command".into(),
                writable_paths: vec![PathBuf::from("/workspace/project/src")],
            },
        };
        let mut command = Command::new(&target.command);

        target.configure_process(&mut command).unwrap();

        let env = command
            .get_envs()
            .filter_map(|(key, value)| {
                value.map(|value| {
                    (
                        key.to_string_lossy().into_owned(),
                        value.to_string_lossy().into_owned(),
                    )
                })
            })
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(
            env.get("KANTERM_DELIVERY_MODE").map(String::as_str),
            Some("packet")
        );
        assert_eq!(
            env.get("KANTERM_APPROVAL_POLICY").map(String::as_str),
            Some("never")
        );
        assert!(env
            .get("KANTERM_WRITABLE_PATHS")
            .is_some_and(|value| value.contains("/workspace/project/src")));
    }

    #[test]
    fn argument_prompt_transport_appends_the_exact_packet() {
        let mut target = CommandTarget {
            name: "cursor".into(),
            agent: Some("cursor".into()),
            repo: Some(PathBuf::from("/workspace/project")),
            command: "cursor-agent".into(),
            args: vec!["--print".into()],
            prompt_transport: PromptTransport::Argument,
            policy: TargetPolicy::default(),
        };
        let packet = "kanterm-agent-work-packet/v1\nprofile: execute\n";
        let mut command = Command::new(&target.command);
        command.args(&target.args);

        assert!(!target.attach_prompt(&mut command, packet));
        assert_eq!(
            command
                .get_args()
                .map(|value| value.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["--print", packet]
        );

        target.prompt_transport = PromptTransport::Stdin;
        let mut command = Command::new(&target.command);
        command.args(&target.args);
        assert!(target.attach_prompt(&mut command, packet));
        assert_eq!(command.get_args().count(), 1);
    }
}
