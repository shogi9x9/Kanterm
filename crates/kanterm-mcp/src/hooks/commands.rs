use super::args::{BridgeConfig, HookInstall};
use super::process::safe_key;

const OWNER_MARKER: &str = "KANTERM_HOOK_OWNER=handoff";

pub(super) fn session_start_command(install: &HookInstall) -> String {
    let mut inner = vec![
        OWNER_MARKER.to_string(),
        "nohup".into(),
        "kanterm-mcp".into(),
        "watch-handoffs".into(),
        "--for-agent".into(),
        install.for_agent.clone(),
        "--claim-token".into(),
        install.claim_token.clone(),
        "--run-dir".into(),
        install.run_dir.display().to_string(),
        "--replace-existing".into(),
    ];
    append_bridge(&mut inner, &install.bridge);
    inner.push(">>".into());
    inner.push(
        install
            .run_dir
            .join(format!("watch.{}.log", safe_key(&install.for_agent)))
            .display()
            .to_string(),
    );
    inner.push("2>&1".into());
    inner.push("&".into());
    format!("sh -c {}", shell_quote(&join_shell_words(&inner)))
}

pub(super) fn session_end_command(install: &HookInstall) -> String {
    let words = vec![
        OWNER_MARKER.to_string(),
        "kanterm-mcp".into(),
        "hooks".into(),
        "stop-watcher".into(),
        "--for-agent".into(),
        install.for_agent.clone(),
        "--run-dir".into(),
        install.run_dir.display().to_string(),
    ];
    join_shell_words(&words)
}

pub(super) fn stop_command(install: &HookInstall) -> String {
    let mut words = vec![
        OWNER_MARKER.to_string(),
        "kanterm-mcp".into(),
        "watch-handoffs".into(),
        "--for-agent".into(),
        install.for_agent.clone(),
        "--claim-token".into(),
        install.claim_token.clone(),
        "--run-dir".into(),
        install.run_dir.display().to_string(),
        "--once".into(),
        "--skip-if-running".into(),
    ];
    append_bridge(&mut words, &install.bridge);
    join_shell_words(&words)
}

pub(super) fn entry_contains_owner(entry: &serde_json::Value) -> bool {
    entry
        .get("hooks")
        .and_then(serde_json::Value::as_array)
        .map(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(serde_json::Value::as_str)
                    .map(|command| command.contains(OWNER_MARKER))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn append_bridge(words: &mut Vec<String>, bridge: &Option<BridgeConfig>) {
    if let Some(bridge) = bridge {
        words.push("--bridge-command".into());
        words.push(bridge.command.clone());
        for arg in &bridge.args {
            words.push("--bridge-arg".into());
            words.push(arg.clone());
        }
    }
}

fn join_shell_words(words: &[String]) -> String {
    words
        .iter()
        .map(|word| shell_quote(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '=' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
