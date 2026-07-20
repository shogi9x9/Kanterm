use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use super::{ConfigManifest, CONFIG_FILE_NAME};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    Explicit,
    Project,
    Global,
}

impl ConfigScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Project => "project",
            Self::Global => "global",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfigPath {
    pub path: PathBuf,
    pub source: ConfigScope,
    pub manifest: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigResolution {
    pub global_manifest: PathBuf,
    pub project_manifest: PathBuf,
    pub targets: Option<ResolvedConfigPath>,
    pub workflow: Option<ResolvedConfigPath>,
}

pub fn global_config_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("KANTERM_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("KANTERM_HOME") {
        return Ok(PathBuf::from(path));
    }
    let dirs = directories::ProjectDirs::from("", "", "Kanterm")
        .ok_or_else(|| anyhow!("could not determine a config directory"))?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn project_config_manifest(cwd: &Path) -> PathBuf {
    let mut current = if cwd.is_file() {
        cwd.parent().unwrap_or(cwd)
    } else {
        cwd
    };
    loop {
        let candidate = current.join(".kanterm").join(CONFIG_FILE_NAME);
        if candidate.is_file() || current.join(".git").exists() {
            return candidate;
        }
        let Some(parent) = current.parent() else {
            return cwd.join(".kanterm").join(CONFIG_FILE_NAME);
        };
        current = parent;
    }
}

pub fn resolve_config(
    cwd: &Path,
    explicit_targets: Option<PathBuf>,
    explicit_workflow: Option<PathBuf>,
) -> Result<ConfigResolution> {
    resolve_config_with_global(
        cwd,
        &global_config_dir()?,
        explicit_targets,
        explicit_workflow,
    )
}

pub fn resolve_config_with_global(
    cwd: &Path,
    global_dir: &Path,
    explicit_targets: Option<PathBuf>,
    explicit_workflow: Option<PathBuf>,
) -> Result<ConfigResolution> {
    let global_manifest = global_dir.join(CONFIG_FILE_NAME);
    let project_manifest = project_config_manifest(cwd);
    let mut targets = None;
    let mut workflow = None;
    apply_manifest(
        &global_manifest,
        ConfigScope::Global,
        &mut targets,
        &mut workflow,
    )?;
    apply_manifest(
        &project_manifest,
        ConfigScope::Project,
        &mut targets,
        &mut workflow,
    )?;
    if let Some(path) = explicit_targets {
        targets = Some(ResolvedConfigPath {
            path: absolutize(cwd, path),
            source: ConfigScope::Explicit,
            manifest: None,
        });
    }
    if let Some(path) = explicit_workflow {
        workflow = Some(ResolvedConfigPath {
            path: absolutize(cwd, path),
            source: ConfigScope::Explicit,
            manifest: None,
        });
    }
    Ok(ConfigResolution {
        global_manifest,
        project_manifest,
        targets,
        workflow,
    })
}

pub fn validate_config(resolution: &ConfigResolution) -> Result<()> {
    for (kind, entry) in [
        ("targets", resolution.targets.as_ref()),
        ("workflow", resolution.workflow.as_ref()),
    ] {
        if let Some(entry) = entry {
            if !entry.path.is_file() {
                return Err(anyhow!(
                    "resolved {kind} file does not exist: {} (source: {})",
                    entry.path.display(),
                    entry.source.as_str()
                ));
            }
        }
    }
    Ok(())
}

fn apply_manifest(
    path: &Path,
    scope: ConfigScope,
    targets: &mut Option<ResolvedConfigPath>,
    workflow: &mut Option<ResolvedConfigPath>,
) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let manifest = ConfigManifest::load(path)?;
    if let Some(value) = manifest.targets {
        *targets = Some(resolved_from_manifest(path, scope, value));
    }
    if let Some(value) = manifest.workflow {
        *workflow = Some(resolved_from_manifest(path, scope, value));
    }
    Ok(())
}

fn resolved_from_manifest(
    manifest: &Path,
    source: ConfigScope,
    value: PathBuf,
) -> ResolvedConfigPath {
    let base = manifest.parent().unwrap_or_else(|| Path::new("."));
    ResolvedConfigPath {
        path: absolutize(base, value),
        source,
        manifest: Some(manifest.to_path_buf()),
    }
}

fn absolutize(base: &Path, value: PathBuf) -> PathBuf {
    if value.is_absolute() {
        value
    } else {
        base.join(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kanterm-config-{name}-{unique}"))
    }

    #[test]
    fn precedence_is_explicit_then_project_then_global_and_paths_are_manifest_relative() {
        let root = temp_dir("precedence");
        let global = root.join("global");
        let repo = root.join("repo");
        let nested = repo.join("src");
        fs::create_dir_all(&global).unwrap();
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(repo.join(".kanterm")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            global.join(CONFIG_FILE_NAME),
            "version: 1\ntargets: global-targets.yaml\nworkflow: global-workflow.yaml\n",
        )
        .unwrap();
        fs::write(
            repo.join(".kanterm").join(CONFIG_FILE_NAME),
            "version: 1\ntargets: project-targets.yaml\n",
        )
        .unwrap();

        let resolved = resolve_config_with_global(
            &nested,
            &global,
            Some(PathBuf::from("explicit-targets.yaml")),
            None,
        )
        .unwrap();
        assert_eq!(resolved.targets.unwrap().source, ConfigScope::Explicit);
        assert_eq!(
            resolved.workflow.unwrap().path,
            global.join("global-workflow.yaml")
        );
        let project_only = resolve_config_with_global(&nested, &global, None, None).unwrap();
        assert_eq!(
            project_only.targets.unwrap().path,
            repo.join(".kanterm").join("project-targets.yaml")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validation_reports_a_missing_resolved_file_with_its_scope() {
        let root = temp_dir("missing");
        let global = root.join("global");
        fs::create_dir_all(&global).unwrap();
        fs::write(
            global.join(CONFIG_FILE_NAME),
            "version: 1\ntargets: missing.yaml\n",
        )
        .unwrap();
        let resolution = resolve_config_with_global(&root, &global, None, None).unwrap();
        let error = validate_config(&resolution).unwrap_err().to_string();
        assert!(error.contains("resolved targets file does not exist"));
        assert!(error.contains("source: global"));
        let _ = fs::remove_dir_all(root);
    }
}
