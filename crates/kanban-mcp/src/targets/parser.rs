use anyhow::{anyhow, Result};
use std::path::PathBuf;

use super::{CommandTarget, DeliveryTarget, InteractiveTarget, TargetConfig};

#[derive(Debug, Clone, Default)]
struct DraftTarget {
    name: String,
    kind: String,
    agent: Option<String>,
    repo: Option<PathBuf>,
    command: Option<String>,
    args: Vec<String>,
    adapter: Option<String>,
    session: Option<String>,
    pane: Option<String>,
}

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
        if indent == 0 {
            flush_target(&mut config, &mut current)?;
            if text == "targets:" {
                in_targets = true;
            } else {
                return Err(anyhow!("unknown target config field: {text}"));
            }
        } else if indent == 2 && text.starts_with("- ") {
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
            current = Some(DraftTarget {
                name: value.to_string(),
                ..Default::default()
            });
        } else if indent == 4 {
            let target = current
                .as_mut()
                .ok_or_else(|| anyhow!("target field before target item: {text}"))?;
            let Some((key, value)) = split_kv(text) else {
                return Err(anyhow!("invalid target field: {text}"));
            };
            match key {
                "type" => target.kind = value.to_string(),
                "agent" => target.agent = Some(value.to_string()),
                "repo" => target.repo = Some(PathBuf::from(value)),
                "command" => target.command = Some(value.to_string()),
                "args" => target.args = split_words(value)?,
                "adapter" => target.adapter = Some(value.to_string()),
                "session" => target.session = Some(value.to_string()),
                "pane" => target.pane = Some(value.to_string()),
                _ => return Err(anyhow!("unknown target field: {key}")),
            }
        } else {
            return Err(anyhow!("unsupported target config indentation: {line}"));
        }
    }
    flush_target(&mut config, &mut current)?;
    if config.targets.is_empty() {
        return Err(anyhow!("target config must contain at least one target"));
    }
    Ok(config)
}

fn flush_target(config: &mut TargetConfig, current: &mut Option<DraftTarget>) -> Result<()> {
    let Some(target) = current.take() else {
        return Ok(());
    };
    if target.name.trim().is_empty() {
        return Err(anyhow!("target name is required"));
    }
    match target.kind.as_str() {
        "command" => {
            let command = target
                .command
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("target '{}' command is required", target.name))?;
            config.targets.push(DeliveryTarget::Command(CommandTarget {
                name: target.name,
                agent: target.agent,
                repo: target.repo,
                command,
                args: target.args,
            }));
        }
        "interactive" => {
            config
                .targets
                .push(DeliveryTarget::Interactive(InteractiveTarget {
                    name: target.name,
                    agent: target.agent,
                    adapter: target.adapter,
                    session: target.session,
                    pane: target.pane,
                }));
        }
        "" => return Err(anyhow!("target '{}' type is required", target.name)),
        other => return Err(anyhow!("unsupported target type: {other}")),
    }
    Ok(())
}

fn trim_comment(line: &str) -> &str {
    line.split_once('#').map(|(left, _)| left).unwrap_or(line)
}

fn split_kv(text: &str) -> Option<(&str, &str)> {
    let (key, value) = text.split_once(':')?;
    Some((key.trim(), strip_quotes(value.trim())))
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

fn split_words(value: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in value.chars() {
        match (quote, ch) {
            (Some(active), ch) if ch == active => quote = None,
            (None, '"' | '\'') => quote = Some(ch),
            (None, ch) if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if let Some(active) = quote {
        return Err(anyhow!("unterminated quote {active} in args"));
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_command_and_interactive_targets() {
        let config = parse_targets(
            r#"
targets:
  - name: bff-command
    type: command
    agent: bff-agent
    repo: /work/downstream-repo
    command: claude
    args: -p "continue work"
  - name: bff-session
    type: interactive
    agent: bff-agent
    adapter: tmux
    session: bff
    pane: 0
"#,
        )
        .unwrap();
        let DeliveryTarget::Command(command) = config.find("bff-command").unwrap() else {
            panic!("expected command target");
        };
        assert_eq!(command.agent.as_deref(), Some("bff-agent"));
        assert_eq!(
            command.repo.as_deref(),
            Some(Path::new("/work/downstream-repo"))
        );
        assert_eq!(command.args, ["-p", "continue work"]);
        let DeliveryTarget::Interactive(interactive) = config.find("bff-session").unwrap() else {
            panic!("expected interactive target");
        };
        assert_eq!(interactive.adapter.as_deref(), Some("tmux"));
        assert_eq!(interactive.session.as_deref(), Some("bff"));
        assert_eq!(interactive.pane.as_deref(), Some("0"));
    }

    #[test]
    fn parse_rejects_unterminated_args_quote() {
        let err = parse_targets(
            r#"
targets:
  - name: bad
    type: command
    command: echo
    args: "unterminated
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unterminated quote"));
    }
}
