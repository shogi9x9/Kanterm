use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum AgentTaskCommand {
    Run(Box<AgentTaskArgs>),
    Help,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentTaskArgs {
    pub(crate) for_agent: String,
    pub(crate) claim_token: String,
    pub(crate) targets: PathBuf,
    pub(crate) target: String,
    pub(crate) board: String,
    pub(crate) card: String,
    pub(crate) lease_minutes: Option<i64>,
    pub(crate) complete_note: Option<String>,
    pub(crate) workflow: Option<PathBuf>,
    pub(crate) workflow_step: Option<String>,
    pub(crate) workflow_targets: Option<PathBuf>,
    pub(crate) from_agent: Option<String>,
}

pub(crate) fn usage() -> &'static str {
    "usage: kanterm-mcp run-agent-task --for-agent ID --claim-token TOKEN --targets PATH --target NAME --board SLUG --card KEY [--lease-minutes MIN] [--complete-note TEXT] [--workflow PATH --workflow-step NAME --workflow-targets PATH --from-agent ID]"
}

pub(crate) fn parse(args: &[String]) -> Result<AgentTaskCommand> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(AgentTaskCommand::Help);
    }
    let mut parsed = PartialArgs::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--for-agent" => {
                i += 1;
                parsed.for_agent = Some(required_value(args, i, "--for-agent")?.to_string());
            }
            "--claim-token" => {
                i += 1;
                parsed.claim_token = Some(required_value(args, i, "--claim-token")?.to_string());
            }
            "--targets" => {
                i += 1;
                parsed.targets = Some(PathBuf::from(required_value(args, i, "--targets")?));
            }
            "--target" => {
                i += 1;
                parsed.target = Some(required_value(args, i, "--target")?.to_string());
            }
            "--board" => {
                i += 1;
                parsed.board = Some(required_value(args, i, "--board")?.to_string());
            }
            "--card" => {
                i += 1;
                parsed.card = Some(required_value(args, i, "--card")?.to_string());
            }
            "--lease-minutes" => {
                i += 1;
                parsed.lease_minutes = Some(
                    required_value(args, i, "--lease-minutes")?
                        .parse()
                        .context("--lease-minutes must be an integer")?,
                );
            }
            "--complete-note" => {
                i += 1;
                parsed.complete_note =
                    Some(required_value(args, i, "--complete-note")?.to_string());
            }
            "--workflow" => {
                i += 1;
                parsed.workflow = Some(PathBuf::from(required_value(args, i, "--workflow")?));
            }
            "--workflow-step" => {
                i += 1;
                parsed.workflow_step =
                    Some(required_value(args, i, "--workflow-step")?.to_string());
            }
            "--workflow-targets" => {
                i += 1;
                parsed.workflow_targets = Some(PathBuf::from(required_value(
                    args,
                    i,
                    "--workflow-targets",
                )?));
            }
            "--from-agent" => {
                i += 1;
                parsed.from_agent = Some(required_value(args, i, "--from-agent")?.to_string());
            }
            other => return Err(anyhow!("unknown run-agent-task argument: {other}")),
        }
        i += 1;
    }
    Ok(AgentTaskCommand::Run(Box::new(parsed.finish()?)))
}

#[derive(Default)]
struct PartialArgs {
    for_agent: Option<String>,
    claim_token: Option<String>,
    targets: Option<PathBuf>,
    target: Option<String>,
    board: Option<String>,
    card: Option<String>,
    lease_minutes: Option<i64>,
    complete_note: Option<String>,
    workflow: Option<PathBuf>,
    workflow_step: Option<String>,
    workflow_targets: Option<PathBuf>,
    from_agent: Option<String>,
}

impl PartialArgs {
    fn finish(self) -> Result<AgentTaskArgs> {
        Ok(AgentTaskArgs {
            for_agent: required_option(self.for_agent, "--for-agent")?,
            claim_token: required_option(self.claim_token, "--claim-token")?,
            targets: self
                .targets
                .ok_or_else(|| anyhow!("--targets is required"))?,
            target: required_option(self.target, "--target")?,
            board: required_option(self.board, "--board")?,
            card: required_option(self.card, "--card")?,
            lease_minutes: self.lease_minutes,
            complete_note: self.complete_note,
            workflow: self.workflow,
            workflow_step: self.workflow_step,
            workflow_targets: self.workflow_targets,
            from_agent: self.from_agent,
        })
    }
}

fn required_value<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str> {
    args.get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} requires a value"))
}

fn required_option(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_requires_core_fields() {
        let err = parse(&args(&["--for-agent", "b"])).unwrap_err();
        assert!(err.to_string().contains("--claim-token is required"));
    }

    #[test]
    fn parse_accepts_workflow_fields() {
        let AgentTaskCommand::Run(parsed) = parse(&args(&[
            "--for-agent",
            "b#1",
            "--claim-token",
            "secret",
            "--targets",
            "targets.yaml",
            "--target",
            "b-command",
            "--board",
            "board",
            "--card",
            "B-1",
            "--workflow",
            "workflow.yaml",
            "--workflow-step",
            "b-to-c",
            "--workflow-targets",
            "targets.yaml",
            "--from-agent",
            "b",
        ]))
        .unwrap() else {
            panic!("expected run command");
        };
        assert_eq!(parsed.workflow_step.as_deref(), Some("b-to-c"));
        assert_eq!(parsed.from_agent.as_deref(), Some("b"));
    }
}
