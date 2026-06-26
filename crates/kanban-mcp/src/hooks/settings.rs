use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use super::args::{mode_name, HookInstall, Mode};
use super::commands::{
    entry_contains_owner, session_end_command, session_start_command, stop_command,
};

pub(super) fn apply_settings(install: &HookInstall) -> Result<()> {
    let mut settings = read_settings(&install.settings)?;
    strip_owned_hooks(&mut settings);
    match install.mode {
        Mode::Monitor => {
            add_hook(
                &mut settings,
                "SessionStart",
                session_start_command(install),
            );
            add_hook(&mut settings, "SessionEnd", session_end_command(install));
        }
        Mode::Turn => add_hook(&mut settings, "Stop", stop_command(install)),
        Mode::Both => {
            add_hook(
                &mut settings,
                "SessionStart",
                session_start_command(install),
            );
            add_hook(&mut settings, "SessionEnd", session_end_command(install));
            add_hook(&mut settings, "Stop", stop_command(install));
        }
        Mode::Off => {}
    }
    prune_empty(&mut settings);
    write_settings(&install.settings, &settings)
}

pub(super) fn apply_off(settings: &Path) -> Result<()> {
    let mut value = read_settings(settings)?;
    strip_owned_hooks(&mut value);
    prune_empty(&mut value);
    write_settings(settings, &value)
}

pub(super) fn print_status(settings: &Path) -> Result<()> {
    let value = read_settings(settings)?;
    let has_start = has_owned_event(&value, "SessionStart");
    let has_stop = has_owned_event(&value, "Stop");
    let mode = match (has_start, has_stop) {
        (true, true) => Mode::Both,
        (true, false) => Mode::Monitor,
        (false, true) => Mode::Turn,
        (false, false) => Mode::Off,
    };
    println!("mode: {}", mode_name(mode));
    println!("settings: {}", settings.display());
    println!(
        "SessionStart: {}",
        count_event_entries(&value, "SessionStart")
    );
    println!("SessionEnd: {}", count_event_entries(&value, "SessionEnd"));
    println!("Stop: {}", count_event_entries(&value, "Stop"));
    Ok(())
}

fn read_settings(path: &Path) -> Result<Value> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            if contents.trim().is_empty() {
                Ok(json!({}))
            } else {
                serde_json::from_str(&contents)
                    .with_context(|| format!("parsing {}", path.display()))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(err) => Err(err).with_context(|| format!("reading {}", path.display())),
    }
}

fn write_settings(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating settings dir {}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

fn add_hook(settings: &mut Value, event: &str, command: String) {
    let event_array = settings
        .as_object_mut()
        .expect("settings root object")
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("hooks object")
        .entry(event)
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("event array");
    event_array.push(json!({
        "hooks": [
            {
                "type": "command",
                "command": command
            }
        ]
    }));
}

fn strip_owned_hooks(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    for event in ["SessionStart", "SessionEnd", "Stop"] {
        let Some(entries) = hooks.get_mut(event).and_then(Value::as_array_mut) else {
            continue;
        };
        entries.retain(|entry| !entry_contains_owner(entry));
    }
}

fn prune_empty(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    for event in ["SessionStart", "SessionEnd", "Stop"] {
        let empty = hooks
            .get(event)
            .and_then(Value::as_array)
            .map(Vec::is_empty)
            .unwrap_or(false);
        if empty {
            hooks.remove(event);
        }
    }
    if hooks.is_empty() {
        settings
            .as_object_mut()
            .expect("settings root object")
            .remove("hooks");
    }
}

fn has_owned_event(settings: &Value, event: &str) -> bool {
    settings
        .pointer(&format!("/hooks/{event}"))
        .and_then(Value::as_array)
        .map(|entries| entries.iter().any(entry_contains_owner))
        .unwrap_or(false)
}

fn count_event_entries(settings: &Value, event: &str) -> usize {
    settings
        .pointer(&format!("/hooks/{event}"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn install_is_idempotent_and_preserves_unowned_hooks() {
        let install = HookInstall {
            mode: Mode::Both,
            settings: PathBuf::from("unused"),
            for_agent: "claude#abc123".into(),
            claim_token: "secret".into(),
            run_dir: PathBuf::from("/tmp/kanterm-run"),
            bridge: None,
        };
        let mut settings = json!({
            "hooks": {
                "Stop": [
                    {"hooks": [{"type": "command", "command": "echo keep"}]},
                    {"hooks": [{"type": "command", "command": "KANTERM_HOOK_OWNER=handoff old"}]}
                ]
            }
        });
        strip_owned_hooks(&mut settings);
        match install.mode {
            Mode::Both => {
                add_hook(
                    &mut settings,
                    "SessionStart",
                    session_start_command(&install),
                );
                add_hook(&mut settings, "SessionEnd", session_end_command(&install));
                add_hook(&mut settings, "Stop", stop_command(&install));
            }
            _ => unreachable!(),
        }
        strip_owned_hooks(&mut settings);
        add_hook(
            &mut settings,
            "SessionStart",
            session_start_command(&install),
        );
        add_hook(&mut settings, "SessionEnd", session_end_command(&install));
        add_hook(&mut settings, "Stop", stop_command(&install));

        assert_eq!(count_event_entries(&settings, "SessionStart"), 1);
        assert_eq!(count_event_entries(&settings, "SessionEnd"), 1);
        assert_eq!(count_event_entries(&settings, "Stop"), 2);
    }
}
