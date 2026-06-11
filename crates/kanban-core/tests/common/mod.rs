use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique temp DB path per test (no external rand/tmpfile crates).
pub fn temp_db(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kanban-it-{tag}-{}-{ts}-{n}.db",
        std::process::id()
    ))
}

pub struct TempDb(pub PathBuf);

impl Drop for TempDb {
    fn drop(&mut self) {
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", self.0.display()));
        }
    }
}
