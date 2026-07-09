use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Default)]
pub(super) struct Workflow {
    pub(super) name: String,
    pub(super) initial_step: Option<String>,
    pub(super) steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct WorkflowStep {
    pub(super) name: String,
    pub(super) agent: Option<String>,
    pub(super) on_complete: Option<WorkflowAction>,
}

#[derive(Debug, Clone)]
pub(super) enum WorkflowAction {
    SendHandoff(SendHandoffRule),
}

#[derive(Debug, Clone, Default)]
pub(super) struct SendHandoffRule {
    pub(super) target: Option<String>,
    pub(super) to_agent: Option<String>,
    pub(super) repo: Option<String>,
    pub(super) subject: String,
    pub(super) body: Option<String>,
}

pub(super) fn parse_workflow(source: &str) -> Result<Workflow> {
    let mut workflow = Workflow::default();
    let mut current_step: Option<WorkflowStep> = None;
    let mut path: Vec<&str> = Vec::new();
    for raw in source.lines() {
        let line = trim_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        let text = line.trim();
        if indent == 0 {
            flush_step(&mut workflow, &mut current_step)?;
            path.clear();
            if text == "steps:" {
                path.push("steps");
            } else if let Some((key, value)) = split_kv(text) {
                match key {
                    "name" => workflow.name = value.to_string(),
                    "initial_step" => workflow.initial_step = Some(value.to_string()),
                    _ => return Err(anyhow!("unknown workflow field: {key}")),
                }
            } else {
                return Err(anyhow!("invalid workflow line: {text}"));
            }
        } else if indent == 2 && text.starts_with("- ") {
            if path.first() != Some(&"steps") {
                return Err(anyhow!("step outside steps list: {text}"));
            }
            flush_step(&mut workflow, &mut current_step)?;
            let Some((key, value)) = split_kv(text.trim_start_matches("- ").trim()) else {
                return Err(anyhow!("step list item must start with name: {text}"));
            };
            if key != "name" {
                return Err(anyhow!("step list item must start with name, got {key}"));
            }
            current_step = Some(WorkflowStep {
                name: value.to_string(),
                ..Default::default()
            });
            path.truncate(1);
            path.push("step");
        } else if indent == 4 {
            let step = current_step
                .as_mut()
                .ok_or_else(|| anyhow!("step field before step item: {text}"))?;
            if text == "on_complete:" {
                path.truncate(2);
                path.push("on_complete");
            } else if let Some((key, value)) = split_kv(text) {
                match key {
                    "agent" => step.agent = Some(value.to_string()),
                    _ => return Err(anyhow!("unknown step field: {key}")),
                }
            } else {
                return Err(anyhow!("invalid step line: {text}"));
            }
        } else if indent == 6 {
            if path.last() != Some(&"on_complete") {
                return Err(anyhow!("action outside on_complete: {text}"));
            }
            if text == "send_handoff:" {
                current_step.as_mut().expect("current step").on_complete =
                    Some(WorkflowAction::SendHandoff(SendHandoffRule::default()));
                path.push("send_handoff");
            } else {
                return Err(anyhow!("unsupported workflow action: {text}"));
            }
        } else if indent == 8 {
            if path.last() != Some(&"send_handoff") {
                return Err(anyhow!("send_handoff field outside action: {text}"));
            }
            let Some((key, value)) = split_kv(text) else {
                return Err(anyhow!("invalid send_handoff line: {text}"));
            };
            let Some(WorkflowAction::SendHandoff(rule)) = current_step
                .as_mut()
                .expect("current step")
                .on_complete
                .as_mut()
            else {
                return Err(anyhow!("send_handoff field before action"));
            };
            match key {
                "target" => rule.target = Some(value.to_string()),
                "to_agent" => rule.to_agent = Some(value.to_string()),
                "repo" => rule.repo = Some(value.to_string()),
                "subject" => rule.subject = value.to_string(),
                "body" => rule.body = Some(value.to_string()),
                _ => return Err(anyhow!("unknown send_handoff field: {key}")),
            }
        } else {
            return Err(anyhow!("unsupported indentation: {line}"));
        }
    }
    flush_step(&mut workflow, &mut current_step)?;
    validate_workflow(&workflow)?;
    Ok(workflow)
}

fn validate_workflow(workflow: &Workflow) -> Result<()> {
    if workflow.name.trim().is_empty() {
        return Err(anyhow!("workflow name is required"));
    }
    if workflow.steps.is_empty() {
        return Err(anyhow!("workflow must contain at least one step"));
    }
    for step in &workflow.steps {
        if let Some(WorkflowAction::SendHandoff(rule)) = &step.on_complete {
            if rule.to_agent.as_deref().unwrap_or("").trim().is_empty()
                && rule.target.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(anyhow!(
                    "step '{}' send_handoff.to_agent or send_handoff.target is required",
                    step.name
                ));
            }
            if rule.subject.trim().is_empty() {
                return Err(anyhow!(
                    "step '{}' send_handoff.subject is required",
                    step.name
                ));
            }
        }
    }
    Ok(())
}

fn flush_step(workflow: &mut Workflow, current_step: &mut Option<WorkflowStep>) -> Result<()> {
    if let Some(step) = current_step.take() {
        if step.name.trim().is_empty() {
            return Err(anyhow!("step name is required"));
        }
        workflow.steps.push(step);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_send_handoff_workflow() {
        let workflow = parse_workflow(
            r#"
name: ms-to-bff
initial_step: implement_ms
steps:
  - name: implement_ms
    agent: ms-agent
    on_complete:
      send_handoff:
        to_agent: bff-agent
        repo: /work/downstream-repo
        subject: Continue {{card}}
        body: Continue {{workflow}} {{step}} in {{repo}}
"#,
        )
        .unwrap();
        assert_eq!(workflow.name, "ms-to-bff");
        assert_eq!(workflow.initial_step.as_deref(), Some("implement_ms"));
        assert_eq!(workflow.steps[0].name, "implement_ms");
        let Some(WorkflowAction::SendHandoff(rule)) = &workflow.steps[0].on_complete else {
            panic!("missing send_handoff rule");
        };
        assert_eq!(rule.to_agent.as_deref(), Some("bff-agent"));
        assert_eq!(rule.repo.as_deref(), Some("/work/downstream-repo"));
    }

    #[test]
    fn parse_target_based_handoff_workflow() {
        let workflow = parse_workflow(
            r#"
name: routed
steps:
  - name: first
    on_complete:
      send_handoff:
        target: bff-command
        subject: Continue via {{target}}
"#,
        )
        .unwrap();
        let Some(WorkflowAction::SendHandoff(rule)) = &workflow.steps[0].on_complete else {
            panic!("missing send_handoff rule");
        };
        assert_eq!(rule.target.as_deref(), Some("bff-command"));
        assert_eq!(rule.to_agent, None);
    }
}
