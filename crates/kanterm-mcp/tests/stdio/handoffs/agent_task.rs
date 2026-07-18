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
    let sent = s.call(
        6,
        "send_handoff",
        json!({
            "from_agent": "a",
            "to_agent": "b",
            "subject": "A says hi to B",
            "body": "Say hi to B and continue to C."
        }),
    );
    let handoff_id = response_field(&sent, "handoff_sent:").to_string();

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
            "--verify-command",
            "true",
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
    let handoff = s.call(10, "get_handoff", json!({"id": handoff_id}));
    assert!(handoff.contains("status: completed"), "got: {handoff}");
    assert!(handoff.contains("result:\nHI_A_TO_B"), "got: {handoff}");

    let store = kanterm_core::Store::open(&db).unwrap();
    let attempts = store.agent_task_attempts(&handoff_id).unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].status, "agent_succeeded");
    assert_eq!(attempts[0].packet_profile, "execute");
    assert!(attempts[0]
        .packet_text
        .starts_with("kanterm-agent-work-packet/v1\nprofile: execute\n"));
    assert!(attempts[0]
        .packet_text
        .contains("handoff_request: <<'KANTERM_HANDOFF_REQUEST'"));
    assert!(attempts[0]
        .packet_text
        .contains("Say hi to B and continue to C."));
    assert_eq!(attempts[0].packet_sha256.len(), 64);
}

#[test]
fn target_and_verification_failures_requeue_the_same_handoff_for_resume() {
    let db = Server::fresh_db();
    let mut server = Server::start_at(db.clone());
    server.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Resume","template":"workflow"}),
    );
    let created = server.call(
        3,
        "create_card",
        json!({"board":"resume","title":"retry verification","column":"Todo"}),
    );
    let card_key = created.split_whitespace().nth(1).unwrap().to_string();
    let registered = server.call(
        4,
        "register_agent",
        json!({"requested_name": "worker", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    let sent = server.call(
        5,
        "send_handoff",
        json!({
            "from_agent": "sender",
            "to_agent": "worker",
            "subject": "Resume after verification",
            "body": "Run the work and verify it."
        }),
    );
    let handoff_id = response_field(&sent, "handoff_sent:").to_string();
    let targets_path = temp_path("kanterm-resume-targets", ".yaml");
    std::fs::write(
        &targets_path,
        "targets:\n  - name: worker-command\n    type: command\n    agent: worker\n    repo: /tmp\n    command: false\n",
    )
    .unwrap();

    let first = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "run-agent-task",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "worker-command",
            "--board",
            "resume",
            "--card",
            &card_key,
            "--verify-command",
            "true",
        ])
        .output()
        .expect("run failing target");
    assert!(!first.status.success());
    assert!(String::from_utf8_lossy(&first.stderr).contains("target command exited"));
    let pending = server.call(6, "get_handoff", json!({"id": handoff_id}));
    assert!(pending.contains("status: pending"), "got: {pending}");
    assert!(pending.contains("target command exited"), "got: {pending}");

    std::fs::write(
        &targets_path,
        "targets:\n  - name: worker-command\n    type: command\n    agent: worker\n    repo: /tmp\n    command: printf\n    args: WORK_DONE\n",
    )
    .unwrap();

    let second = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "run-agent-task",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "worker-command",
            "--board",
            "resume",
            "--card",
            &card_key,
            "--verify-command",
            "false",
        ])
        .output()
        .expect("run failing resume verification");
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("verification_failed"));
    let pending = server.call(7, "get_handoff", json!({"id": handoff_id}));
    assert!(pending.contains("status: pending"), "got: {pending}");
    assert!(pending.contains("verification_failed"), "got: {pending}");

    let third = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "run-agent-task",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "worker-command",
            "--board",
            "resume",
            "--card",
            &card_key,
            "--verify-command",
            "true",
        ])
        .output()
        .expect("run passing resume verification");
    let _ = std::fs::remove_file(&targets_path);
    assert!(
        third.status.success(),
        "resume failed: {}",
        String::from_utf8_lossy(&third.stderr)
    );
    assert!(String::from_utf8_lossy(&third.stdout).contains("agent_task: completed"));
    let completed = server.call(8, "get_handoff", json!({"id": handoff_id}));
    assert!(completed.contains("status: completed"), "got: {completed}");

    let store = kanterm_core::Store::open(&db).unwrap();
    let attempts = store.agent_task_attempts(&handoff_id).unwrap();
    assert_eq!(attempts.len(), 3);
    assert_eq!(attempts[0].packet_profile, "execute");
    assert_eq!(attempts[0].status, "agent_failed");
    assert_eq!(attempts[1].packet_profile, "resume");
    assert_eq!(attempts[1].status, "agent_succeeded");
    assert_eq!(attempts[2].packet_profile, "resume");
    assert!(attempts[2].packet_text.contains("## Resume delta"));
    assert!(attempts[2].packet_text.contains("attempt 1 [agent_failed]"));
    assert!(attempts[2]
        .packet_text
        .contains("attempt 2 [agent_succeeded]"));
}

#[test]
fn invalid_workflow_is_rejected_before_agent_execution() {
    let db = Server::fresh_db();
    let mut server = Server::start_at(db.clone());
    server.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Preflight","template":"workflow"}),
    );
    let created = server.call(
        3,
        "create_card",
        json!({"board":"preflight","title":"validate workflow first","column":"Todo"}),
    );
    let card_key = created.split_whitespace().nth(1).unwrap().to_string();
    let registered = server.call(
        4,
        "register_agent",
        json!({"requested_name": "worker", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    let sent = server.call(
        5,
        "send_handoff",
        json!({
            "from_agent": "sender",
            "to_agent": "worker",
            "subject": "Preflight workflow",
            "body": "Do not run until the workflow is valid."
        }),
    );
    let handoff_id = response_field(&sent, "handoff_sent:").to_string();
    let targets_path = temp_path("kanterm-preflight-targets", ".yaml");
    let workflow_path = temp_path("kanterm-preflight-workflow", ".yaml");
    std::fs::write(
        &targets_path,
        "targets:\n  - name: worker-command\n    type: command\n    agent: worker\n    repo: /tmp\n    command: printf\n    args: SHOULD_NOT_RUN\n",
    )
    .unwrap();
    std::fs::write(
        &workflow_path,
        "name: invalid\ninitial_step: missing\nsteps:\n  - name: present\n    agent: worker\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "run-agent-task",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--targets",
            targets_path.to_str().unwrap(),
            "--target",
            "worker-command",
            "--board",
            "preflight",
            "--card",
            &card_key,
            "--verify-command",
            "true",
            "--workflow",
            workflow_path.to_str().unwrap(),
            "--workflow-targets",
            targets_path.to_str().unwrap(),
        ])
        .output()
        .expect("run task with invalid workflow");
    let _ = std::fs::remove_file(&workflow_path);
    let _ = std::fs::remove_file(&targets_path);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("workflow step 'missing' was not found")
    );
    let card = server.call(
        6,
        "get_card",
        json!({"board": "preflight", "key": card_key}),
    );
    assert!(card.contains("column: Todo"), "got: {card}");
    assert!(card.contains("agent_state: open"), "got: {card}");
    assert!(!card.contains("SHOULD_NOT_RUN"), "got: {card}");
    let handoff = server.call(7, "get_handoff", json!({"id": handoff_id}));
    assert!(handoff.contains("status: pending"), "got: {handoff}");
    let store = kanterm_core::Store::open(&db).unwrap();
    assert!(store.agent_task_attempts(&handoff_id).unwrap().is_empty());
}
