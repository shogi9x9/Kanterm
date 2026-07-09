use crate::now_ms;

pub(crate) fn new_id() -> String {
    // A monotonic-ish unique id without pulling in uuid: time + counter.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:04x}", now_ms(), n & 0xffff)
}
