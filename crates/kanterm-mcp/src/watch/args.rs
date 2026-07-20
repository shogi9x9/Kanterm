use anyhow::{anyhow, Context, Result};
use kanterm_core::resolve_config;
use std::path::PathBuf;

use super::delivery::{delivery_from_target, BridgeCommand, BridgePrompt, Delivery};
use crate::targets::PromptTransport;
use crate::targets::TargetConfig;

const DEFAULT_INTERVAL_MS: u64 = 5_000;

#[derive(Debug, Clone)]
pub(crate) struct WatchArgs {
    pub(crate) for_agent: String,
    pub(crate) claim_token: String,
    pub(crate) interval_ms: u64,
    pub(crate) lease_minutes: Option<i64>,
    pub(crate) once: bool,
    pub(crate) replace_existing: bool,
    pub(crate) skip_if_running: bool,
    pub(crate) run_dir: PathBuf,
    pub(crate) bridge: Option<Delivery>,
}

impl WatchArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self> {
        let mut for_agent = None;
        let mut claim_token = None;
        let mut interval_ms = DEFAULT_INTERVAL_MS;
        let mut lease_minutes = None;
        let mut once = false;
        let mut replace_existing = false;
        let mut skip_if_running = false;
        let mut run_dir = std::env::var_os("KANTERM_RUN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(default_run_dir);
        let mut bridge_program = None;
        let mut bridge_args = Vec::new();
        let mut targets_path = None;
        let mut target_name = None;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--for-agent" => {
                    i += 1;
                    for_agent = Some(required_value(args, i, "--for-agent")?.to_string());
                }
                "--claim-token" => {
                    i += 1;
                    claim_token = Some(required_value(args, i, "--claim-token")?.to_string());
                }
                "--interval-ms" => {
                    i += 1;
                    interval_ms = required_value(args, i, "--interval-ms")?
                        .parse()
                        .context("--interval-ms must be an integer")?;
                }
                "--lease-minutes" => {
                    i += 1;
                    lease_minutes = Some(
                        required_value(args, i, "--lease-minutes")?
                            .parse()
                            .context("--lease-minutes must be an integer")?,
                    );
                }
                "--once" => once = true,
                "--replace-existing" => replace_existing = true,
                "--skip-if-running" => skip_if_running = true,
                "--run-dir" => {
                    i += 1;
                    run_dir = PathBuf::from(required_value(args, i, "--run-dir")?);
                }
                "--bridge-command" => {
                    i += 1;
                    bridge_program = Some(required_value(args, i, "--bridge-command")?.to_string());
                }
                "--bridge-arg" => {
                    i += 1;
                    bridge_args.push(required_value(args, i, "--bridge-arg")?.to_string());
                }
                "--targets" => {
                    i += 1;
                    targets_path = Some(PathBuf::from(required_value(args, i, "--targets")?));
                }
                "--target" => {
                    i += 1;
                    target_name = Some(required_value(args, i, "--target")?.to_string());
                }
                "--help" | "-h" => return Err(anyhow!("{}", usage())),
                other => return Err(anyhow!("unknown watch-handoffs argument: {other}")),
            }
            i += 1;
        }
        if bridge_program.is_some() && target_name.is_some() {
            return Err(anyhow!(
                "--bridge-command and --target cannot be used together"
            ));
        }
        let bridge = match (bridge_program, target_name) {
            (Some(program), None) => Some(Delivery::Bridge(BridgeCommand {
                program,
                args: bridge_args,
                cwd: None,
                prompt: BridgePrompt::Body,
                prompt_transport: PromptTransport::Stdin,
                policy: None,
            })),
            (None, Some(name)) => {
                let cwd = std::env::current_dir().context("determining current directory")?;
                let path = resolve_config(&cwd, targets_path, None)?
                    .targets
                    .ok_or_else(|| {
                        anyhow!(
                            "--targets is required with --target because no targets file is configured; run `kanterm config init --project` or pass --targets PATH"
                        )
                    })?
                    .path;
                let config = TargetConfig::load(&path)?;
                Some(delivery_from_target(config.find(&name)?)?)
            }
            (None, None) => None,
            (Some(_), Some(_)) => unreachable!("checked above"),
        };
        Ok(Self {
            for_agent: required_option(for_agent, "--for-agent")?,
            claim_token: required_option(claim_token, "--claim-token")?,
            interval_ms,
            lease_minutes,
            once,
            replace_existing,
            skip_if_running,
            run_dir,
            bridge,
        })
    }
}

pub(crate) fn usage() -> &'static str {
    "usage: kanterm-mcp watch-handoffs --for-agent ID --claim-token TOKEN [--once] [--replace-existing] [--skip-if-running] [--run-dir DIR] [--interval-ms MS] [--lease-minutes MIN] [--bridge-command PROGRAM --bridge-arg ARG ... | [--targets PATH] --target NAME]"
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

fn default_run_dir() -> PathBuf {
    std::env::temp_dir().join("kanterm").join("run")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_rejects_bridge_command_and_target_together() {
        let err = WatchArgs::parse(&args(&[
            "--for-agent",
            "bff-agent",
            "--claim-token",
            "secret",
            "--bridge-command",
            "cat",
            "--targets",
            "targets.yaml",
            "--target",
            "bff-command",
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("cannot be used together"));
    }

    #[test]
    fn parse_requires_targets_when_target_is_used() {
        let err = WatchArgs::parse(&args(&[
            "--for-agent",
            "bff-agent",
            "--claim-token",
            "secret",
            "--target",
            "bff-command",
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("--targets is required"));
    }
}
