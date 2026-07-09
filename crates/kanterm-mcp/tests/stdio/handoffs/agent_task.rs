use super::*;

#[test]
fn run_agent_task_completes_card_and_triggers_next_handoff() {
    let db = Server::fresh_db();
    let mut s = Server::start_at(db.clone());
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Chain","template":"workflow"}),
    );
    let created = s.call(
        3,
        "create_card",
        json!({"board":"chain","title":"B handles incoming work","column":"Todo"}),
    );
    let card_key = created
        .split_whitespace()
        .nth(1)
        .expect("created card key")
        .to_string();
    let b_agent = s.call(
        4,
        "register_agent",
        json!({"requested_name": "b", "lease_minutes": 30}),
    );
    let b_identity = response_field(&b_agent, "assigned_identity:").to_string();
    let b_token = response_field(&b_agent, "claim_token:").to_string();
    let c_agent = s.call(
        5,
        "register_agent",
        json!({"requested_name": "c", "lease_minutes": 30}),
    );
    let c_identity = response_field(&c_agent, "assigned_identity:").to_string();
    s.call(
        6,
        "send_handoff",
        json!({
            "from_agent": "a",
            "to_agent": "b",
            "subject": "A says hi to B",
            "body": "Say hi to B and continue to C."
        }),
    );

    let targets_path = temp_path("kanterm-runner-targets", ".yaml");
    let workflow_path = temp_path("kanterm-runner-workflow", ".yaml");
    std::fs::write(
        &targets_path,
        r#"
targets:
  - name: b-command
    type: command
    agent: b
    repo: /tmp
    command: printf
    args: HI_A_TO_B
  - name: c-command
    type: command
    agent: c
    repo: /tmp
    command: printf
    args: HI_B_TO_C
"#,
    )
    .unwrap();
    std::fs::write(
        &workflow_path,
        r#"
name: chain-runner
initial_step: b-to-c
steps:
  - name: b-to-c
    agent: b
    on_complete:
      send_handoff:
        target: c-command
        subject: B says hi to C
        body: B completed {{board}}/{{card}} and says hi to C.
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "run-agent-task",
            "--for-agent",
            &b_identity,
            "--claim-token",
            &b_token,
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "b-command",
            "--board",
            "chain",
            "--card",
            &card_key,
            "--workflow",
            workflow_path.to_str().unwrap(),
            "--workflow-targets",
            targets_path.to_str().unwrap(),
            "--workflow-step",
            "b-to-c",
            "--from-agent",
            "b",
        ])
        .output()
        .expect("run agent task");
    let _ = std::fs::remove_file(&workflow_path);
    let _ = std::fs::remove_file(&targets_path);
    assert!(
        output.status.success(),
        "runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("agent_task: completed"), "got: {stdout}");
    assert!(stdout.contains("HI_A_TO_B"), "got: {stdout}");
    assert!(stdout.contains("workflow_triggered:"), "got: {stdout}");
    assert!(stdout.contains("to_agent: c"), "got: {stdout}");

    let b_inbox = s.call(7, "list_handoffs", json!({"for_agent": b_identity}));
    assert_eq!(b_inbox, "no handoffs");
    let c_inbox = s.call(8, "list_handoffs", json!({"for_agent": c_identity}));
    assert!(c_inbox.contains("B says hi to C"), "got: {c_inbox}");
    let card = s.call(9, "get_card", json!({"board": "chain", "key": card_key}));
    assert!(card.contains("state: done"), "got: {card}");
    assert!(card.contains("HI_A_TO_B"), "got: {card}");
}
