mod commands;
mod editor;

use anyhow::{Context, Result};
use kanterm_core::global_config_dir;

pub fn run(args: &[String]) -> Result<bool> {
    if args.first().map(String::as_str) != Some("config") {
        return Ok(false);
    }
    let cwd = std::env::current_dir().context("determining current directory")?;
    let global_dir = global_config_dir()?;
    commands::run(&args[1..], &cwd, &global_dir)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
