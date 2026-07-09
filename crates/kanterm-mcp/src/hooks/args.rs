use anyhow::{anyhow, Result};
use std::path::PathBuf;

use super::process::default_run_dir;

const DEFAULT_SETTINGS: &str = ".claude/settings.local.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Monitor,
    Turn,
    Both,
    Off,
}

#[derive(Debug, Clone)]
pub(crate) enum HookCommand {
    Install(HookInstall),
    Uninstall { settings: PathBuf },
    Status { settings: PathBuf },
    StopWatcher { for_agent: String, run_dir: PathBuf },
    Help,
}

#[derive(Debug, Clone)]
pub(crate) struct HookInstall {
    pub(super) mode: Mode,
    pub(super) settings: PathBuf,
    pub(super) for_agent: String,
    pub(super) claim_token: String,
    pub(super) run_dir: PathBuf,
    pub(super) bridge: Option<BridgeConfig>,
}

#[derive(Debug, Clone)]
pub(super) struct BridgeConfig {
    pub(super) command: String,
    pub(super) args: Vec<String>,
}

pub(crate) fn usage() -> &'static str {
    "usage: kanterm-mcp hooks install --runtime claude-code --mode monitor|turn|both|off --for-agent ID --claim-token TOKEN [--settings PATH] [--run-dir DIR] [--bridge-command PROGRAM --bridge-arg ARG ...]\n       kanterm-mcp hooks status [--settings PATH]\n       kanterm-mcp hooks uninstall [--settings PATH]\n       kanterm-mcp hooks stop-watcher --for-agent ID [--run-dir DIR]"
}

pub(crate) fn parse(args: &[String]) -> Result<HookCommand> {
    let Some(action) = args.first().map(String::as_str) else {
        return Ok(HookCommand::Help);
    };
    match action {
        "install" => parse_install(&args[1..]).map(HookCommand::Install),
        "uninstall" => Ok(HookCommand::Uninstall {
            settings: parse_settings_only(&args[1..])?,
        }),
        "status" => Ok(HookCommand::Status {
            settings: parse_settings_only(&args[1..])?,
        }),
        "stop-watcher" => parse_stop_watcher(&args[1..]),
        "--help" | "-h" => Ok(HookCommand::Help),
        other => Err(anyhow!("unknown hooks action: {other}")),
    }
}

pub(super) fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Monitor => "monitor",
        Mode::Turn => "turn",
        Mode::Both => "both",
        Mode::Off => "off",
    }
}

fn parse_install(args: &[String]) -> Result<HookInstall> {
    let mut runtime = None;
    let mut mode = None;
    let mut settings = PathBuf::from(DEFAULT_SETTINGS);
    let mut for_agent = None;
    let mut claim_token = None;
    let mut run_dir = default_run_dir();
    let mut bridge_command = None;
    let mut bridge_args = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--runtime" => {
                i += 1;
                runtime = Some(required_value(args, i, "--runtime")?.to_string());
            }
            "--mode" => {
                i += 1;
                mode = Some(parse_mode(required_value(args, i, "--mode")?)?);
            }
            "--settings" => {
                i += 1;
                settings = PathBuf::from(required_value(args, i, "--settings")?);
            }
            "--for-agent" => {
                i += 1;
                for_agent = Some(required_value(args, i, "--for-agent")?.to_string());
            }
            "--claim-token" => {
                i += 1;
                claim_token = Some(required_value(args, i, "--claim-token")?.to_string());
            }
            "--run-dir" => {
                i += 1;
                run_dir = PathBuf::from(required_value(args, i, "--run-dir")?);
            }
            "--bridge-command" => {
                i += 1;
                bridge_command = Some(required_value(args, i, "--bridge-command")?.to_string());
            }
            "--bridge-arg" => {
                i += 1;
                bridge_args.push(required_value(args, i, "--bridge-arg")?.to_string());
            }
            "--help" | "-h" => return Err(anyhow!("{}", usage())),
            other => return Err(anyhow!("unknown hooks install argument: {other}")),
        }
        i += 1;
    }
    match runtime.as_deref().unwrap_or("claude-code") {
        "claude-code" => {}
        other => return Err(anyhow!("unsupported hooks runtime: {other}")),
    }
    let mode = mode.unwrap_or(Mode::Both);
    if mode != Mode::Off && for_agent.is_none() {
        return Err(anyhow!("--for-agent is required unless --mode off"));
    }
    if mode != Mode::Off && claim_token.is_none() {
        return Err(anyhow!("--claim-token is required unless --mode off"));
    }
    let bridge = bridge_command.map(|command| BridgeConfig {
        command,
        args: bridge_args,
    });
    Ok(HookInstall {
        mode,
        settings,
        for_agent: for_agent.unwrap_or_default(),
        claim_token: claim_token.unwrap_or_default(),
        run_dir,
        bridge,
    })
}

fn parse_stop_watcher(args: &[String]) -> Result<HookCommand> {
    let mut for_agent = None;
    let mut run_dir = default_run_dir();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--for-agent" => {
                i += 1;
                for_agent = Some(required_value(args, i, "--for-agent")?.to_string());
            }
            "--run-dir" => {
                i += 1;
                run_dir = PathBuf::from(required_value(args, i, "--run-dir")?);
            }
            "--help" | "-h" => return Ok(HookCommand::Help),
            other => return Err(anyhow!("unknown hooks stop-watcher argument: {other}")),
        }
        i += 1;
    }
    Ok(HookCommand::StopWatcher {
        for_agent: required_option(for_agent, "--for-agent")?,
        run_dir,
    })
}

fn parse_settings_only(args: &[String]) -> Result<PathBuf> {
    let mut settings = PathBuf::from(DEFAULT_SETTINGS);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--settings" => {
                i += 1;
                settings = PathBuf::from(required_value(args, i, "--settings")?);
            }
            "--runtime" => {
                i += 1;
                let runtime = required_value(args, i, "--runtime")?;
                if runtime != "claude-code" {
                    return Err(anyhow!("unsupported hooks runtime: {runtime}"));
                }
            }
            "--help" | "-h" => return Err(anyhow!("{}", usage())),
            other => return Err(anyhow!("unknown hooks argument: {other}")),
        }
        i += 1;
    }
    Ok(settings)
}

fn parse_mode(mode: &str) -> Result<Mode> {
    match mode {
        "monitor" => Ok(Mode::Monitor),
        "turn" => Ok(Mode::Turn),
        "both" => Ok(Mode::Both),
        "off" => Ok(Mode::Off),
        _ => Err(anyhow!("mode must be monitor, turn, both, or off")),
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
