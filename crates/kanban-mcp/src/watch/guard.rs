use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub(super) struct WatchGuard {
    pidfile: PathBuf,
    ready_file: PathBuf,
    pid: u32,
}

impl WatchGuard {
    pub(super) fn claim(
        run_dir: &Path,
        for_agent: &str,
        replace_existing: bool,
        skip_if_running: bool,
    ) -> Result<Option<Self>> {
        fs::create_dir_all(run_dir)
            .with_context(|| format!("creating watcher run dir {}", run_dir.display()))?;
        let key = safe_key(for_agent);
        let pidfile = run_dir.join(format!("watch.{key}.pid"));
        let ready_file = run_dir.join(format!("watch.{key}.ready"));
        let pid = std::process::id();
        if let Some(existing) = read_pid(&pidfile)? {
            if process_exists(existing) {
                if skip_if_running {
                    return Ok(None);
                }
                if !replace_existing {
                    return Err(anyhow!(
                        "watcher for '{for_agent}' is already running as pid {existing}; pass --replace-existing to replace it"
                    ));
                }
                if process_matches(existing, "kanterm-mcp", "watch-handoffs") {
                    let _ = Command::new("kill").arg(existing.to_string()).status();
                }
                wait_for_exit(existing, Duration::from_millis(500));
            }
        }
        fs::write(&pidfile, format!("{pid}\n"))
            .with_context(|| format!("writing watcher pidfile {}", pidfile.display()))?;
        let _ = fs::remove_file(&ready_file);
        Ok(Some(Self {
            pidfile,
            ready_file,
            pid,
        }))
    }

    pub(super) fn mark_ready(&self) -> Result<()> {
        fs::write(&self.ready_file, format!("{}\n", self.pid))
            .with_context(|| format!("writing watcher ready file {}", self.ready_file.display()))
    }
}

impl Drop for WatchGuard {
    fn drop(&mut self) {
        if read_pid(&self.pidfile).ok().flatten() == Some(self.pid) {
            let _ = fs::remove_file(&self.pidfile);
            let _ = fs::remove_file(&self.ready_file);
        }
    }
}

fn safe_key(value: &str) -> String {
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

fn wait_for_exit(pid: u32, timeout: Duration) {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if !process_exists(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
