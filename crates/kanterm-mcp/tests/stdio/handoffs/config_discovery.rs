use super::*;

#[test]
fn config_manifest_drives_headless_workflow_and_target_delivery() {
    let db = Server::fresh_db();
    let root = temp_path("kanterm-config-headless", "");
    let config_dir = root.join("config");
    let target_repo = root.join("target-repo");
    let run_dir = root.join("run");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::create_dir_all(&target_repo).unwrap();
    std::fs::write(
        config_dir.join("config.yaml"),
        "version: 1\ntargets: targets.yaml\nworkflow: workflow.yaml\n",
    )
    .unwrap();
    std::fs::write(
        config_dir.join("targets.yaml"),
        format!(
            "targets:\n  - name: config-worker\n    type: command\n    agent: config-worker\n    repo: {}\n    command: tee\n    args: delivered.txt\n",
            target_repo.display()
        ),
    )
    .unwrap();
    std::fs::write(
        config_dir.join("workflow.yaml"),
        "name: config-headless\ninitial_step: dispatch\nsteps:\n  - name: dispatch\n    agent: producer\n    on_complete:\n      send_handoff:\n        target: config-worker\n        subject: Config-driven delivery\n        body: Workflow and target paths came from config.yaml.\n",
    )
    .unwrap();

    let mut server = Server::start_at(db.clone());
    let registered = server.call(
        2,
        "register_agent",
        json!({"requested_name": "config-worker", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();

    let workflow = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .env("KANTERM_CONFIG_DIR", &config_dir)
        .args(["run-workflow", "--from-agent", "producer"])
        .output()
        .expect("run workflow from config");
    assert!(
        workflow.status.success(),
        "config workflow failed: {}",
        String::from_utf8_lossy(&workflow.stderr)
    );
    let workflow_output = String::from_utf8_lossy(&workflow.stdout);
    assert!(workflow_output.contains("workflow: config-headless"));
    assert!(workflow_output.contains("action: send_handoff"));
    assert!(workflow_output.contains("to_agent: config-worker"));

    let inbox = server.call(3, "list_handoffs", json!({"for_agent": identity}));
    assert!(inbox.contains("Config-driven delivery"), "got: {inbox}");
    let handoff_id = inbox.split_whitespace().next().unwrap().to_string();

    let watcher = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .env("KANTERM_CONFIG_DIR", &config_dir)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--once",
            "--target",
            "config-worker",
        ])
        .output()
        .expect("run watcher from config");
    assert!(
        watcher.status.success(),
        "config watcher failed: {}",
        String::from_utf8_lossy(&watcher.stderr)
    );

    let delivered = std::fs::read_to_string(target_repo.join("delivered.txt")).unwrap();
    assert!(delivered.starts_with("kanterm-agent-work-packet/v1\nprofile: execute\n"));
    assert!(delivered.contains("Config-driven delivery"));
    assert!(delivered.contains("Workflow and target paths came from config.yaml."));
    let detail = server.call(4, "get_handoff", json!({"id": handoff_id}));
    assert!(detail.contains("status: completed"), "got: {detail}");
    assert!(detail.contains("result:\nkanterm-agent-work-packet/v1"));

    let _ = std::fs::remove_dir_all(root);
}
