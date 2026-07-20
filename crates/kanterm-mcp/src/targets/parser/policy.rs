use anyhow::{anyhow, Result};
use std::path::{Component, Path, PathBuf};

use super::draft::DraftTarget;
use crate::targets::TargetPolicy;

pub(super) fn command_policy(target: &DraftTarget) -> Result<TargetPolicy> {
    let defaults = TargetPolicy::default();
    let delivery = target.delivery.clone().unwrap_or(defaults.delivery);
    require_one_of(&delivery, "delivery", &["packet"])?;
    let environment = target.environment.clone().unwrap_or(defaults.environment);
    require_one_of(&environment, "environment", &["inherit", "clean"])?;
    let network = target.network.clone().unwrap_or(defaults.network);
    require_one_of(&network, "network", &["inherit"])?;
    let workspace = target.workspace.clone().unwrap_or(defaults.workspace);
    require_one_of(&workspace, "workspace", &["repo-write"])?;
    let approval = target.approval.clone().unwrap_or(defaults.approval);
    require_one_of(&approval, "approval", &["external", "never", "on-request"])?;
    let verification = target.verification.clone().unwrap_or(defaults.verification);
    require_one_of(&verification, "verification", &["command", "none"])?;

    let writable_paths = if target.writable_paths.is_empty() {
        Vec::new()
    } else {
        let repo = target.repo.as_ref().ok_or_else(|| {
            anyhow!(
                "target '{}' writable_paths requires a repo workspace",
                target.name
            )
        })?;
        target
            .writable_paths
            .iter()
            .map(|path| resolve_writable_path(target, repo, path))
            .collect::<Result<Vec<_>>>()?
    };

    Ok(TargetPolicy {
        delivery,
        environment,
        network,
        workspace,
        approval,
        verification,
        writable_paths,
    })
}

fn resolve_writable_path(target: &DraftTarget, repo: &Path, path: &Path) -> Result<PathBuf> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(outside_repo_error(target, repo, path));
    }
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    };
    if !resolved.starts_with(repo) {
        return Err(outside_repo_error(target, repo, path));
    }
    validate_existing_ancestor(target, repo, path, &resolved)?;
    Ok(resolved)
}

fn validate_existing_ancestor(
    target: &DraftTarget,
    repo: &Path,
    configured: &Path,
    resolved: &Path,
) -> Result<()> {
    if !repo.exists() {
        return Ok(());
    }
    let canonical_repo = repo.canonicalize()?;
    let mut ancestor = resolved;
    while !ancestor.exists() {
        let Some(parent) = ancestor.parent() else {
            return Err(outside_repo_error(target, repo, configured));
        };
        ancestor = parent;
    }
    if !ancestor.canonicalize()?.starts_with(&canonical_repo) {
        return Err(outside_repo_error(target, repo, configured));
    }
    Ok(())
}

fn outside_repo_error(target: &DraftTarget, repo: &Path, path: &Path) -> anyhow::Error {
    anyhow!(
        "target '{}' writable path '{}' must stay within repo '{}'",
        target.name,
        path.display(),
        repo.display()
    )
}

fn require_one_of(value: &str, field: &str, allowed: &[&str]) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(anyhow!(
            "unsupported target {field}: {value}; supported: {}",
            allowed.join(", ")
        ))
    }
}
