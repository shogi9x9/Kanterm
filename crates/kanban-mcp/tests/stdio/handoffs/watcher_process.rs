use super::*;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn watch_handoffs_writes_ready_file_and_rejects_duplicate() {
    let db = Server::fresh_db();
    let run_dir = temp_path("kanterm-watch-run", "");
    let mut s = Server::start_at(db.clone());
    let registered = s.call(
        2,
        "register_agent",
        json!({"requested_name": "claude", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();

    let mut first = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--interval-ms",
            "50",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn first watcher");

    let key = identity.replace('#', "_");
    let ready_file = run_dir.join(format!("watch.{key}.ready"));
    wait_for_path(&ready_file);
    assert!(ready_file.exists());

    let duplicate = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--once",
        ])
        .output()
        .expect("run duplicate watcher");
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already running"));

    let skipped = Command::new(env!("CARGO_BIN_EXE_kanterm-mcp"))
        .env("KANBAN_DB", &db)
        .args([
            "watch-handoffs",
            "--for-agent",
            &identity,
            "--claim-token",
            &token,
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--once",
            "--skip-if-running",
        ])
        .output()
        .expect("run skip watcher");
    assert!(
        skipped.status.success(),
        "skip watcher should exit cleanly: {}",
        String::from_utf8_lossy(&skipped.stderr)
    );
    assert!(skipped.stdout.is_empty());

    let _ = first.kill();
    let _ = first.wait();
    let _ = std::fs::remove_dir_all(&run_dir);
}

fn wait_for_path(path: &std::path::Path) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {}", path.display());
}
