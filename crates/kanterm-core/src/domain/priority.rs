pub const PRIORITY_LOW: i64 = 0;
pub const PRIORITY_NORMAL: i64 = 1;
pub const PRIORITY_HIGH: i64 = 2;

pub fn priority_label(p: i64) -> &'static str {
    match p {
        PRIORITY_LOW => "low",
        PRIORITY_HIGH => "high",
        _ => "normal",
    }
}

pub fn priority_badge(p: i64) -> &'static str {
    match p {
        PRIORITY_LOW => "[L]",
        PRIORITY_HIGH => "[H]",
        _ => "[M]",
    }
}
