use super::*;

#[test]
fn run_workflow_creates_card_linked_handoff() {
    let db = Server::fresh_db();
    let mut s = Server::start_at(db.clone());
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"MS","template":"workflow"}),
    );
    let created = s.call(
        3,
        "create_card",
        json!({"board":"ms","title":"finish microservice slice","column":"Todo"}),
    );
    assert!(created.contains("created MS-1"), "got: {created}");

    let workflow_path = temp_path("kanterm-workflow", ".yaml");
    let targets_path = temp_path("kanterm-targets", ".yaml");
    std::fs::write(
        &targets_path,
        r#"
targets:
  - name: bff-command
    type: command
    agent: bff-agent
    repo: /work/downstream-repo
    command: claude
    args: -p
"#,
    )
    .unwrap();
    std::fs::write(
        &workflow_path,
        r#"
name: ms-to-bff
initial_step: implement_ms
steps:
  - name: implement_ms
    agent: ms-agent
    on_complete:
      send_handoff:
        target: bff-command
        subject: Continue {{card}} for {{workflow}}
        body: Continue {{step}} from {{from_agent}} to {{to_agent}} via {{target}} into {{repo}} on {{board}}/{{card}}
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "run-workflow",
            "--workflow",
            workflow_path.to_str().unwrap(),
            "--from-agent",
            "ms-agent",
            "--targets",
            targets_path.to_str().unwrap(),
            "--board",
            "ms",
            "--card",
            "MS-1",
        ])
        .output()
        .expect("run workflow");
    let _ = std::fs::remove_file(&workflow_path);
    let _ = std::fs::remove_file(&targets_path);
    assert!(
        output.status.success(),
        "workflow failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("action: send_handoff"), "got: {stdout}");
    assert!(stdout.contains("to_agent: bff-agent"), "got: {stdout}");

    let registered = s.call(
        4,
        "register_agent",
        json!({"requested_name": "bff-agent", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    let inbox = s.call(5, "list_handoffs", json!({"for_agent": identity}));
    assert!(
        inbox.contains("Continue MS-1 for ms-to-bff"),
        "got: {inbox}"
    );
    assert!(inbox.contains("MS-1"), "got: {inbox}");
    let handoff_id = inbox.split_whitespace().next().unwrap().to_string();
    let claimed = s.call(
        6,
        "claim_handoff",
        json!({"id": handoff_id, "claimant": response_field(&registered, "assigned_identity:"), "claim_token": token}),
    );
    assert!(
        claimed.contains("Continue implement_ms from ms-agent to bff-agent via bff-command"),
        "got: {claimed}"
    );
    assert!(
        claimed.contains("/work/downstream-repo on ms/MS-1"),
        "got: {claimed}"
    );
}
