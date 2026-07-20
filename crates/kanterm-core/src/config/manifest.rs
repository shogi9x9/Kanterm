use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::CONFIG_VERSION;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigManifest {
    pub version: u32,
    pub targets: Option<PathBuf>,
    pub workflow: Option<PathBuf>,
}

impl ConfigManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("reading config manifest {}", path.display()))?;
        Self::parse(&source).with_context(|| format!("parsing config manifest {}", path.display()))
    }

    pub fn parse(source: &str) -> Result<Self> {
        let mut version = None;
        let mut targets = None;
        let mut workflow = None;
        for (index, raw) in source.lines().enumerate() {
            let line = raw.split_once('#').map(|(left, _)| left).unwrap_or(raw);
            if line.trim().is_empty() {
                continue;
            }
            if line.chars().next().is_some_and(char::is_whitespace) {
                return Err(anyhow!(
                    "line {}: config fields must be top-level",
                    index + 1
                ));
            }
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| anyhow!("line {}: expected key: value", index + 1))?;
            let key = key.trim();
            let value = strip_quotes(value.trim());
            match key {
                "version" => {
                    if version.is_some() {
                        return Err(anyhow!("line {}: duplicate version", index + 1));
                    }
                    version = Some(value.parse::<u32>().with_context(|| {
                        format!("line {}: version must be an integer", index + 1)
                    })?);
                }
                "targets" => set_path(&mut targets, value, "targets", index + 1)?,
                "workflow" => set_path(&mut workflow, value, "workflow", index + 1)?,
                other => {
                    return Err(anyhow!(
                        "line {}: unknown config field '{other}'",
                        index + 1
                    ))
                }
            }
        }
        let version = version.ok_or_else(|| anyhow!("config version is required"))?;
        if version != CONFIG_VERSION {
            return Err(anyhow!(
                "unsupported config version {version}; supported: {CONFIG_VERSION}"
            ));
        }
        Ok(Self {
            version,
            targets,
            workflow,
        })
    }
}

fn set_path(slot: &mut Option<PathBuf>, value: &str, name: &str, line: usize) -> Result<()> {
    if slot.is_some() {
        return Err(anyhow!("line {line}: duplicate {name}"));
    }
    if value.is_empty() {
        return Err(anyhow!("line {line}: {name} path cannot be empty"));
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn strip_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_a_supported_version_and_known_fields() {
        let parsed = ConfigManifest::parse(
            "version: 1\ntargets: targets.yaml\nworkflow: 'workflows/default.yaml'\n",
        )
        .unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.targets, Some(PathBuf::from("targets.yaml")));
        assert_eq!(
            parsed.workflow,
            Some(PathBuf::from("workflows/default.yaml"))
        );
        assert!(ConfigManifest::parse("version: 2\n")
            .unwrap_err()
            .to_string()
            .contains("unsupported config version"));
        assert!(ConfigManifest::parse("version: 1\nsecret: value\n")
            .unwrap_err()
            .to_string()
            .contains("unknown config field 'secret'"));
    }
}
