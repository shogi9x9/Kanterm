use super::*;

#[test]
fn watch_handoffs_delivers_jsonl_and_leaves_completion_to_the_receiver() {
    let db = Server::fresh_db();
    let mut s = Server::start_at(db.clone());
    let registered = s.call(
        2,
        "register_agent",
        json!({"requested_name": "claude", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    let sent = s.call(
        3,
        "send_handoff",
        json!({
            "from_agent": "codex#sender",
            "to_agent": "claude",
            "subject": "Watch me",
            "body": "handoff body for stdout"
        }),
    );
    let handoff_id = response_field(&sent, "handoff_sent:").to_string();
    let run_dir = temp_path("kanterm-jsonl-watcher", "");

    let output = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--once",
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--interval-ms",
            "1",
        ])
        .output()
        .expect("run handoff watcher");
    assert!(
        output.status.success(),
        "watcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let line = String::from_utf8(output.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(payload["id"], handoff_id);
    assert_eq!(payload["body"], "handoff body for stdout");

    let repeated = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--once",
            "--run-dir",
            run_dir.to_str().unwrap(),
        ])
        .output()
        .expect("rerun handoff watcher");
    assert!(repeated.status.success());
    assert!(repeated.stdout.is_empty(), "claimed work was redelivered");

    let detail = s.call(4, "get_handoff", json!({"id": handoff_id}));
    assert!(detail.contains("status: claimed"), "got: {detail}");
    assert!(!detail.contains("result:"), "got: {detail}");
}
