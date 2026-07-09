use std::path::PathBuf;

use anyhow::Result;
use kanterm_core::Store;

use crate::{agent_task, hooks, watch, workflow};

pub(crate) fn run_if_cli_command(args: &[String]) -> Result<bool> {
    match args.get(1).map(String::as_str) {
        Some("hooks") => {
            let hook_command = hooks::parse(&args[2..])?;
            hooks::run(hook_command)?;
        }
        Some("run-workflow") => run_workflow(&args[2..])?,
        Some("run-agent-task") => run_agent_task(&args[2..])?,
        Some("watch-handoffs") => run_watch_handoffs(&args[2..])?,
        _ => return Ok(false),
    }
    Ok(true)
}

fn run_workflow(args: &[String]) -> Result<()> {
    let workflow_command = workflow::parse(args)?;
    if matches!(workflow_command, workflow::WorkflowCommand::Help) {
        println!("{}", workflow::usage());
        return Ok(());
    }
    let mut store = Store::open(&db_path()?)?;
    if let workflow::WorkflowCommand::Run(run_args) = workflow_command {
        workflow::run(&mut store, run_args)?;
    }
    Ok(())
}

fn run_agent_task(args: &[String]) -> Result<()> {
    let task_command = agent_task::parse(args)?;
    if matches!(task_command, agent_task::AgentTaskCommand::Help) {
        println!("{}", agent_task::usage());
        return Ok(());
    }
    let mut store = Store::open(&db_path()?)?;
    if let agent_task::AgentTaskCommand::Run(run_args) = task_command {
        agent_task::run(&mut store, *run_args)?;
    }
    Ok(())
}

fn run_watch_handoffs(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{}", watch::usage());
        return Ok(());
    }
    let watch_args = watch::WatchArgs::parse(args)?;
    let mut store = Store::open(&db_path()?)?;
    watch::run(&mut store, watch_args)
}

fn db_path() -> Result<PathBuf> {
    match std::env::var_os("KANBAN_DB") {
        Some(p) => Ok(PathBuf::from(p)),
        None => Store::default_db_path(),
    }
}
