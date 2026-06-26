use super::*;

#[test]
fn update_card_complete_note_triggers_workflow_handoff() {
    let db = Server::fresh_db();
    let mut s = Server::start_at(db);
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"MS","template":"workflow"}),
    );
    let created = s.call(
        3,
        "create_card",
        json!({"board":"ms","title":"complete and trigger","column":"Todo"}),
    );
    assert!(created.contains("created MS-1"), "got: {created}");

    let targets_path = temp_path("kanterm-update-targets", ".yaml");
    let workflow_path = temp_path("kanterm-update-workflow", ".yaml");
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
name: update-complete-trigger
initial_step: upstream-done
steps:
  - name: upstream-done
    agent: ms-agent
    on_complete:
      send_handoff:
        target: bff-command
        subject: Continue {{card}} from update_card
        body: update_card completed {{board}}/{{card}} by {{from_agent}} for {{to_agent}}
"#,
    )
    .unwrap();

    let updated = s.call(
        4,
        "update_card",
        json!({
            "board": "ms",
            "key": "MS-1",
            "complete_note": "done upstream",
            "workflow": workflow_path,
            "workflow_targets": targets_path,
            "workflow_from_agent": "ms-agent"
        }),
    );
    let _ = std::fs::remove_file(&workflow_path);
    let _ = std::fs::remove_file(&targets_path);
    assert!(updated.contains("updated MS-1"), "got: {updated}");
    assert!(updated.contains("workflow_triggered:"), "got: {updated}");
    assert!(updated.contains("action: send_handoff"), "got: {updated}");
    assert!(updated.contains("to_agent: bff-agent"), "got: {updated}");

    let registered = s.call(
        5,
        "register_agent",
        json!({"requested_name": "bff-agent", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();
    let inbox = s.call(6, "list_handoffs", json!({"for_agent": identity}));
    assert!(
        inbox.contains("Continue MS-1 from update_card"),
        "got: {inbox}"
    );
    let handoff_id = inbox.split_whitespace().next().unwrap().to_string();
    let claimed = s.call(
        7,
        "claim_handoff",
        json!({"id": handoff_id, "claimant": response_field(&registered, "assigned_identity:"), "claim_token": token}),
    );
    assert!(
        claimed.contains("update_card completed ms/MS-1 by ms-agent for bff-agent"),
        "got: {claimed}"
    );
}

#[test]
fn update_card_rejects_workflow_trigger_without_complete_note() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"MS","template":"workflow"}),
    );
    s.call(
        3,
        "create_card",
        json!({"board":"ms","title":"not complete","column":"Todo"}),
    );
    let err = s.call_error(
        4,
        "update_card",
        json!({
            "board": "ms",
            "key": "MS-1",
            "workflow": "/tmp/kanterm.workflow.yaml"
        }),
    );
    assert!(
        err.contains("workflow trigger fields require complete_note"),
        "got: {err}"
    );
}
