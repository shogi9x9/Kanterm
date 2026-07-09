use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn stop_watcher(run_dir: &Path, for_agent: &str) -> Result<()> {
    let pidfile = run_dir.join(format!("watch.{}.pid", safe_key(for_agent)));
    let Some(pid) = read_pid(&pidfile)? else {
        return Ok(());
    };
    if process_exists(pid) && process_matches(pid, "kanterm-mcp", "watch-handoffs") {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }
    Ok(())
}

pub(super) fn default_run_dir() -> PathBuf {
    std::env::temp_dir().join("kanterm").join("run")
}

pub(super) fn safe_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn read_pid(path: &Path) -> Result<Option<u32>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents.trim().parse().ok()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("reading pidfile {}", path.display())),
    }
}

fn process_exists(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn process_matches(pid: u32, needle_a: &str, needle_b: &str) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .map(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains(needle_a)
                && String::from_utf8_lossy(&output.stdout).contains(needle_b)
        })
        .unwrap_or(false)
}
