use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum WorkflowCommand {
    Run(RunWorkflowArgs),
    Help,
}

#[derive(Debug, Clone)]
pub(crate) struct RunWorkflowArgs {
    pub(crate) workflow: PathBuf,
    pub(crate) event: String,
    pub(crate) step: Option<String>,
    pub(crate) from_agent: String,
    pub(crate) board: Option<String>,
    pub(crate) card: Option<String>,
    pub(crate) targets: Option<PathBuf>,
}

pub(crate) fn usage() -> &'static str {
    "usage: kanterm-mcp run-workflow --workflow PATH --from-agent ID [--event complete] [--step NAME] [--board SLUG] [--card KEY] [--targets PATH]"
}

pub(crate) fn parse(args: &[String]) -> Result<WorkflowCommand> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(WorkflowCommand::Help);
    }
    let mut workflow = None;
    let mut event = "complete".to_string();
    let mut step = None;
    let mut from_agent = None;
    let mut board = None;
    let mut card = None;
    let mut targets = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--workflow" => {
                i += 1;
                workflow = Some(PathBuf::from(required_value(args, i, "--workflow")?));
            }
            "--event" => {
                i += 1;
                event = required_value(args, i, "--event")?.to_string();
            }
            "--step" => {
                i += 1;
                step = Some(required_value(args, i, "--step")?.to_string());
            }
            "--from-agent" => {
                i += 1;
                from_agent = Some(required_value(args, i, "--from-agent")?.to_string());
            }
            "--board" => {
                i += 1;
                board = Some(required_value(args, i, "--board")?.to_string());
            }
            "--card" => {
                i += 1;
                card = Some(required_value(args, i, "--card")?.to_string());
            }
            "--targets" => {
                i += 1;
                targets = Some(PathBuf::from(required_value(args, i, "--targets")?));
            }
            other => return Err(anyhow!("unknown run-workflow argument: {other}")),
        }
        i += 1;
    }
    Ok(WorkflowCommand::Run(RunWorkflowArgs {
        workflow: workflow.ok_or_else(|| anyhow!("--workflow is required"))?,
        event,
        step,
        from_agent: required_option(from_agent, "--from-agent")?,
        board,
        card,
        targets,
    }))
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
