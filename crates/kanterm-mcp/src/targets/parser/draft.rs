use anyhow::{anyhow, Result};
use std::path::PathBuf;

use super::policy::command_policy;
use super::syntax::split_words;
use crate::targets::{CommandTarget, DeliveryTarget, InteractiveTarget, PromptTransport};

#[derive(Debug, Clone, Default)]
pub(super) struct DraftTarget {
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) agent: Option<String>,
    pub(super) repo: Option<PathBuf>,
    pub(super) command: Option<String>,
    pub(super) args: Vec<String>,
    pub(super) model: Option<String>,
    pub(super) adapter: Option<String>,
    pub(super) session: Option<String>,
    pub(super) pane: Option<String>,
    pub(super) socket: Option<PathBuf>,
    pub(super) delivery: Option<String>,
    pub(super) environment: Option<String>,
    pub(super) network: Option<String>,
    pub(super) workspace: Option<String>,
    pub(super) approval: Option<String>,
    pub(super) verification: Option<String>,
    pub(super) writable_paths: Vec<PathBuf>,
}

impl DraftTarget {
    pub(super) fn named(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Self::default()
        }
    }

    pub(super) fn set_field(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "type" => self.kind = value.to_string(),
            "agent" => self.agent = Some(value.to_string()),
            "repo" => self.repo = Some(PathBuf::from(value)),
            "command" => self.command = Some(value.to_string()),
            "args" => self.args = split_words(value)?,
            "model" => self.model = Some(value.to_string()),
            "adapter" => self.adapter = Some(value.to_string()),
            "session" => self.session = Some(value.to_string()),
            "pane" => self.pane = Some(value.to_string()),
            "socket" => self.socket = Some(PathBuf::from(value)),
            "delivery" => self.delivery = Some(value.to_string()),
            "environment" => self.environment = Some(value.to_string()),
            "network" => self.network = Some(value.to_string()),
            "workspace" => self.workspace = Some(value.to_string()),
            "approval" => self.approval = Some(value.to_string()),
            "verification" => self.verification = Some(value.to_string()),
            "writable_paths" => {
                self.writable_paths = split_words(value)?.into_iter().map(PathBuf::from).collect();
            }
            _ => return Err(anyhow!("unknown target field: {key}")),
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Result<DeliveryTarget> {
        if self.name.trim().is_empty() {
            return Err(anyhow!("target name is required"));
        }
        if self.socket.is_some()
            && (self.kind != "interactive" || self.adapter.as_deref() != Some("kanpty"))
        {
            return Err(anyhow!(
                "target '{}' socket is supported only for type: interactive with adapter: kanpty",
                self.name
            ));
        }
        match self.kind.as_str() {
            "command" => self.command_target(),
            "cursor" => self.cursor_target(),
            "interactive" => self.interactive_target(),
            "" => Err(anyhow!("target '{}' type is required", self.name)),
            other => Err(anyhow!("unsupported target type: {other}")),
        }
    }

    fn command_target(self) -> Result<DeliveryTarget> {
        if self.model.is_some() {
            return Err(anyhow!(
                "target '{}' model is supported only for type: cursor",
                self.name
            ));
        }
        let policy = command_policy(&self)?;
        let command = self
            .command
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("target '{}' command is required", self.name))?;
        Ok(DeliveryTarget::Command(CommandTarget {
            name: self.name,
            agent: self.agent,
            repo: self.repo,
            command,
            args: self.args,
            prompt_transport: PromptTransport::Stdin,
            policy,
        }))
    }

    fn cursor_target(self) -> Result<DeliveryTarget> {
        if self.command.is_some() || !self.args.is_empty() {
            return Err(anyhow!(
                "target '{}' type: cursor defines its command and args; use type: command for custom invocation",
                self.name
            ));
        }
        let repo = self
            .repo
            .clone()
            .ok_or_else(|| anyhow!("target '{}' type: cursor requires repo", self.name))?;
        let policy = command_policy(&self)?;
        if policy.environment != "inherit" {
            return Err(anyhow!(
                "target '{}' type: cursor requires environment: inherit so Cursor authentication remains available",
                self.name
            ));
        }
        if policy.approval == "on-request" {
            return Err(anyhow!(
                "target '{}' type: cursor is headless and does not support approval: on-request; use external or never",
                self.name
            ));
        }
        let mut args = vec![
            "--print".into(),
            "--output-format".into(),
            "text".into(),
            "--trust".into(),
            "--workspace".into(),
            repo.to_string_lossy().into_owned(),
        ];
        if policy.approval == "never" {
            args.push("--force".into());
        }
        if let Some(model) = self.model.filter(|value| !value.trim().is_empty()) {
            args.push("--model".into());
            args.push(model);
        }
        Ok(DeliveryTarget::Command(CommandTarget {
            name: self.name,
            agent: self.agent.or_else(|| Some("cursor".into())),
            repo: Some(repo),
            command: "cursor-agent".into(),
            args,
            prompt_transport: PromptTransport::Argument,
            policy,
        }))
    }

    fn interactive_target(self) -> Result<DeliveryTarget> {
        if self.model.is_some() {
            return Err(anyhow!(
                "target '{}' model is supported only for type: cursor",
                self.name
            ));
        }
        if self.adapter.as_deref() == Some("kanpty") {
            if self.socket.as_ref().is_some_and(|path| !path.is_absolute()) {
                return Err(anyhow!(
                    "target '{}' adapter: kanpty socket must be an absolute path or omitted to use the default",
                    self.name
                ));
            }
            if self
                .session
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(anyhow!(
                    "target '{}' adapter: kanpty requires session",
                    self.name
                ));
            }
            if self.pane.is_some() {
                return Err(anyhow!(
                    "target '{}' adapter: kanpty addresses a session ID or alias and does not accept pane",
                    self.name
                ));
            }
        }
        Ok(DeliveryTarget::Interactive(InteractiveTarget {
            name: self.name,
            agent: self.agent,
            adapter: self.adapter,
            session: self.session,
            pane: self.pane,
            socket: self.socket,
        }))
    }
}
