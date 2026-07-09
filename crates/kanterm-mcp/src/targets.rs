mod parser;

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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
}

#[derive(Debug, Clone)]
pub(crate) struct InteractiveTarget {
    pub(crate) name: String,
    pub(crate) agent: Option<String>,
    pub(crate) adapter: Option<String>,
    pub(crate) session: Option<String>,
    pub(crate) pane: Option<String>,
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
