mod draft;
mod policy;
mod syntax;

use anyhow::{anyhow, Result};

use self::draft::DraftTarget;
use self::syntax::{split_kv, trim_comment};
use super::TargetConfig;

pub(super) fn parse_targets(source: &str) -> Result<TargetConfig> {
    let mut config = TargetConfig::default();
    let mut current: Option<DraftTarget> = None;
    let mut in_targets = false;
    for raw in source.lines() {
        let line = trim_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        let text = line.trim();
        match indent {
            0 => {
                flush_target(&mut config, &mut current)?;
                if text != "targets:" {
                    return Err(anyhow!("unknown target config field: {text}"));
                }
                in_targets = true;
            }
            2 if text.starts_with("- ") => {
                if !in_targets {
                    return Err(anyhow!("target item outside targets list: {text}"));
                }
                flush_target(&mut config, &mut current)?;
                let Some((key, value)) = split_kv(text.trim_start_matches("- ").trim()) else {
                    return Err(anyhow!("target list item must start with name: {text}"));
                };
                if key != "name" {
                    return Err(anyhow!("target list item must start with name, got {key}"));
                }
                current = Some(DraftTarget::named(value));
            }
            4 => {
                let target = current
                    .as_mut()
                    .ok_or_else(|| anyhow!("target field before target item: {text}"))?;
                let Some((key, value)) = split_kv(text) else {
                    return Err(anyhow!("invalid target field: {text}"));
                };
                target.set_field(key, value)?;
            }
            _ => return Err(anyhow!("unsupported target config indentation: {line}")),
        }
    }
    flush_target(&mut config, &mut current)?;
    if config.targets.is_empty() {
        return Err(anyhow!("target config must contain at least one target"));
    }
    Ok(config)
}

fn flush_target(config: &mut TargetConfig, current: &mut Option<DraftTarget>) -> Result<()> {
    if let Some(target) = current.take() {
        config.targets.push(target.finish()?);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
