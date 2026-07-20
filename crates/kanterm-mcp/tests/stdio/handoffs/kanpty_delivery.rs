use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn watcher_pastes_packet_to_kanpty_and_requeues_transient_delivery_failure() {
    let db = Server::fresh_db();
    let root = temp_path("kanterm-kanpty-adapter", "");
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let args_capture = root.join("args.txt");
    let stdin_capture = root.join("stdin.txt");
    let fake_kanpty = bin.join("kanpty");
    write_fake_kanpty(&fake_kanpty, true);
    let targets_path = root.join("targets.yaml");
    std::fs::write(
        &targets_path,
        r#"
targets:
  - name: claude-interactive
    type: interactive
    agent: claude
    adapter: kanpty
    session: claude-board-a
    socket: /tmp/kanpty-test.sock
"#,
    )
    .unwrap();
    let mut server = Server::start_at(db.clone());
    let registered = server.call(
        2,
        "register_agent",
        json!({"requested_name": "claude", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    let first_id = send_handoff(&mut server, 3, "Interactive delivery");
    let path = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    )))
    .unwrap();

    let output = run_watcher(
        &db,
        &identity,
        &token,
        &targets_path,
        &path,
        &args_capture,
        &stdin_capture,
    );
    assert!(
        output.status.success(),
        "watcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let delivered_args = std::fs::read_to_string(&args_capture).unwrap();
    let delivered_packet = std::fs::read_to_string(&stdin_capture).unwrap();
    assert_eq!(
        delivered_args,
        "--socket\n/tmp/kanpty-test.sock\npaste\n--enter\nclaude-board-a\n"
    );
    assert!(!delivered_args.contains("kanterm-agent-work-packet/v1"));
    assert!(delivered_packet.starts_with("kanterm-agent-work-packet/v1"));
    assert!(delivered_packet.contains("Interactive delivery"));
    assert!(delivered_packet.contains("continue the implementation"));
    let delivered = server.call(4, "get_handoff", json!({"id": first_id}));
    assert!(delivered.contains("status: claimed"), "got: {delivered}");

    write_fake_kanpty(&fake_kanpty, false);
    let second_id = send_handoff(&mut server, 5, "Retry delivery");
    let failed = run_watcher(
        &db,
        &identity,
        &token,
        &targets_path,
        &path,
        &args_capture,
        &stdin_capture,
    );
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("exit status: 9"));
    let pending = server.call(6, "get_handoff", json!({"id": second_id}));
    assert!(pending.contains("status: pending"), "got: {pending}");
    assert!(pending.contains("exit status: 9"), "got: {pending}");

    let _ = std::fs::remove_dir_all(root);
}

fn write_fake_kanpty(path: &std::path::Path, succeed: bool) {
    let exit = if succeed { 0 } else { 9 };
    std::fs::write(
        path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$KANPTY_ARGS_CAPTURE\"\ncat > \"$KANPTY_STDIN_CAPTURE\"\nexit {exit}\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn send_handoff(server: &mut Server, id: i64, subject: &str) -> String {
    let sent = server.call(
        id,
        "send_handoff",
        json!({
            "from_agent": "codex#sender",
            "to_agent": "claude",
            "subject": subject,
            "body": "continue the implementation"
        }),
    );
    response_field(&sent, "handoff_sent:").to_string()
}

#[allow(clippy::too_many_arguments)]
fn run_watcher(
    db: &str,
    identity: &str,
    token: &str,
    targets_path: &std::path::Path,
    path: &std::ffi::OsStr,
    args_capture: &std::path::Path,
    stdin_capture: &std::path::Path,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", db)
        .env("KANPTY_ARGS_CAPTURE", args_capture)
        .env("KANPTY_STDIN_CAPTURE", stdin_capture)
        .env("PATH", path)
        .args([
            "watch-handoffs",
            "--for-agent",
            identity,
            "--claim-token",
            token,
            "--once",
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "claude-interactive",
        ])
        .output()
        .expect("run kanpty target watcher")
}
