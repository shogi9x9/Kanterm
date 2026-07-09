use super::*;

#[test]
fn hooks_install_status_and_uninstall_are_idempotent() {
    let settings = temp_path("kanterm-hooks", ".json");
    std::fs::write(
        &settings,
        r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"echo keep"}]}]}}"#,
    )
    .unwrap();

    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
            .args([
                "hooks",
                "install",
                "--runtime",
                "claude-code",
                "--mode",
                "both",
                "--for-agent",
                "claude#abc123",
                "--claim-token",
                "secret",
                "--settings",
                settings.to_str().unwrap(),
                "--run-dir",
                "/tmp/kanterm-hooks-test",
            ])
            .output()
            .expect("install hooks");
        assert!(
            output.status.success(),
            "install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(value["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
    assert_eq!(value["hooks"]["SessionEnd"].as_array().unwrap().len(), 1);
    assert_eq!(value["hooks"]["Stop"].as_array().unwrap().len(), 2);
    assert!(value["hooks"]["Stop"].to_string().contains("echo keep"));

    let status = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .args(["hooks", "status", "--settings", settings.to_str().unwrap()])
        .output()
        .expect("status hooks");
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("mode: both"));

    let uninstall = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .args([
            "hooks",
            "uninstall",
            "--settings",
            settings.to_str().unwrap(),
        ])
        .output()
        .expect("uninstall hooks");
    assert!(uninstall.status.success());
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(value["hooks"]["Stop"].as_array().unwrap().len(), 1);
    assert!(value["hooks"].get("SessionStart").is_none());
    assert!(value["hooks"].get("SessionEnd").is_none());
    let _ = std::fs::remove_file(&settings);
}
