mod agent_task;
mod config_discovery;
mod hooks;
mod roundtrip;
mod watcher;
mod watcher_delivery;
mod watcher_process;
mod workflow_runner;
mod workflow_trigger;

use serde_json::json;
use std::process::Command;

use crate::support::{response_field, Server};

fn temp_path(prefix: &str, suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        suffix
    ))
}
