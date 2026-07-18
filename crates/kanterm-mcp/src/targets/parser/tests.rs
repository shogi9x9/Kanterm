use super::parse_targets;
use crate::targets::{DeliveryTarget, PromptTransport};
use std::path::{Path, PathBuf};

#[test]
fn parse_command_and_interactive_targets() {
    let config = parse_targets(
        r#"
targets:
  - name: bff-command
    type: command
    agent: bff-agent
    repo: /work/downstream-repo
    command: agent-cli
    args: -p "continue work"
    delivery: packet
    environment: clean
    network: inherit
    workspace: repo-write
    approval: never
    verification: command
    writable_paths: src tests
  - name: bff-session
    type: interactive
    agent: bff-agent
    adapter: tmux
    session: bff
    pane: 0
"#,
    )
    .unwrap();
    let DeliveryTarget::Command(command) = config.find("bff-command").unwrap() else {
        panic!("expected command target");
    };
    assert_eq!(command.agent.as_deref(), Some("bff-agent"));
    assert_eq!(
        command.repo.as_deref(),
        Some(Path::new("/work/downstream-repo"))
    );
    assert_eq!(command.args, ["-p", "continue work"]);
    assert_eq!(command.policy.delivery, "packet");
    assert_eq!(command.policy.environment, "clean");
    assert_eq!(command.policy.approval, "never");
    assert_eq!(
        command.policy.writable_paths,
        [
            PathBuf::from("/work/downstream-repo/src"),
            PathBuf::from("/work/downstream-repo/tests")
        ]
    );
    let DeliveryTarget::Interactive(interactive) = config.find("bff-session").unwrap() else {
        panic!("expected interactive target");
    };
    assert_eq!(interactive.adapter.as_deref(), Some("tmux"));
    assert_eq!(interactive.session.as_deref(), Some("bff"));
    assert_eq!(interactive.pane.as_deref(), Some("0"));
}

#[test]
fn parse_rejects_unterminated_args_quote() {
    let err = parse_targets(
        r#"
targets:
  - name: bad
    type: command
    command: echo
    args: "unterminated
"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("unterminated quote"));
}

#[test]
fn parse_cursor_target_builds_a_headless_argument_adapter() {
    let config = parse_targets(
        r#"
targets:
  - name: cursor-worker
    type: cursor
    repo: /work/project
    model: composer-2.5
    approval: never
    verification: command
"#,
    )
    .unwrap();
    let DeliveryTarget::Command(command) = config.find("cursor-worker").unwrap() else {
        panic!("expected command target");
    };
    assert_eq!(command.agent.as_deref(), Some("cursor"));
    assert_eq!(command.command, "cursor-agent");
    assert_eq!(
        command.args,
        [
            "--print",
            "--output-format",
            "text",
            "--trust",
            "--workspace",
            "/work/project",
            "--force",
            "--model",
            "composer-2.5"
        ]
    );
    assert_eq!(command.prompt_transport, PromptTransport::Argument);
}

#[test]
fn parse_cursor_target_rejects_interactive_or_auth_losing_policy() {
    let on_request = parse_targets(
        r#"
targets:
  - name: cursor-worker
    type: cursor
    repo: /work/project
    approval: on-request
"#,
    )
    .unwrap_err();
    assert!(on_request
        .to_string()
        .contains("does not support approval: on-request"));

    let clean = parse_targets(
        r#"
targets:
  - name: cursor-worker
    type: cursor
    repo: /work/project
    environment: clean
"#,
    )
    .unwrap_err();
    assert!(clean.to_string().contains("requires environment: inherit"));
}

#[test]
fn parse_rejects_unsupported_or_escaping_isolation() {
    let network = parse_targets(
        r#"
targets:
  - name: unsupported-network
    type: command
    command: agent-cli
    network: deny
"#,
    )
    .unwrap_err();
    assert!(network
        .to_string()
        .contains("unsupported target network: deny"));

    let writable = parse_targets(
        r#"
targets:
  - name: escaping-path
    type: command
    repo: /workspace/project
    command: agent-cli
    writable_paths: /workspace/other
"#,
    )
    .unwrap_err();
    assert!(writable
        .to_string()
        .contains("must stay within repo '/workspace/project'"));

    let traversal = parse_targets(
        r#"
targets:
  - name: parent-traversal
    type: command
    repo: /workspace/project
    command: agent-cli
    writable_paths: src/../../outside
"#,
    )
    .unwrap_err();
    assert!(traversal
        .to_string()
        .contains("must stay within repo '/workspace/project'"));
}

#[cfg(unix)]
#[test]
fn parse_rejects_writable_path_through_an_existing_symlink() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "kanterm-policy-symlink-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let repo = root.join("repo");
    let outside = root.join("outside");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    symlink(&outside, repo.join("escape")).unwrap();
    let source = format!(
        "targets:\n  - name: symlink-escape\n    type: command\n    repo: {}\n    command: agent-cli\n    writable_paths: escape/output\n",
        repo.display()
    );

    let error = parse_targets(&source).unwrap_err().to_string();
    assert!(error.contains("must stay within repo"));
    let _ = std::fs::remove_dir_all(root);
}
