use super::*;
use std::io::Read;
use std::process::Stdio;

#[test]
fn watch_handoffs_delivers_to_command_target() {
    let db = Server::fresh_db();
    let target_repo = temp_path("kanterm-target-repo", "");
    std::fs::create_dir_all(&target_repo).unwrap();
    let target_output = target_repo.join("handoff.txt");
    let targets_path = temp_path("kanterm-watch-targets", ".yaml");
    std::fs::write(
        &targets_path,
        format!(
            r#"
targets:
  - name: bff-command
    type: command
    agent: bff-agent
    repo: {}
    command: tee
    args: handoff.txt
"#,
            target_repo.display()
        ),
    )
    .unwrap();
    let mut s = Server::start_at(db.clone());
    let registered = s.call(
        2,
        "register_agent",
        json!({"requested_name": "bff-agent", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    s.call(
        3,
        "send_handoff",
        json!({
            "from_agent": "ms-agent",
            "to_agent": "bff-agent",
            "subject": "Target delivery",
            "body": "handoff body for target command"
        }),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--once",
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "bff-command",
        ])
        .output()
        .expect("run target watcher");
    let _ = std::fs::remove_file(&targets_path);
    assert!(
        output.status.success(),
        "target watcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let delivered = std::fs::read_to_string(&target_output).unwrap();
    let _ = std::fs::remove_dir_all(&target_repo);
    assert!(delivered.contains("Kanterm handoff received."));
    assert!(delivered.contains("subject: Target delivery"));
    assert!(delivered.contains("handoff body for target command"));

    let empty = s.call(4, "list_handoffs", json!({"for_agent": identity}));
    assert_eq!(empty, "no handoffs");
}

#[test]
fn watch_handoffs_bridges_body_to_command() {
    let db = Server::fresh_db();
    let target_repo = temp_path("kanterm-bridge-repo", "");
    std::fs::create_dir_all(&target_repo).unwrap();
    let mut s = Server::start_at(db.clone());
    let registered = s.call(
        2,
        "register_agent",
        json!({"requested_name": "claude", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    s.call(
        3,
        "send_handoff",
        json!({
            "from_agent": "codex#sender",
            "to_agent": "claude",
            "subject": "Bridge me",
            "body": "handoff body for bridge"
        }),
    );
    let bridge_script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/kanterm-bridge-file-inbox.sh")
        .canonicalize()
        .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--once",
            "--bridge-command",
            bridge_script.to_str().unwrap(),
            "--bridge-arg",
            "--repo",
            "--bridge-arg",
            target_repo.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bridge watcher");
    let status = child.wait().unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(status.success(), "bridge watcher failed: {stderr}");
    let bridged = std::fs::read_to_string(
        target_repo.join(".kanterm/inbox").join(
            std::fs::read_dir(target_repo.join(".kanterm/inbox"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .file_name(),
        ),
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(&target_repo);
    assert!(bridged.contains("Bridge me"));
    assert!(bridged.contains("handoff body for bridge"));
}
