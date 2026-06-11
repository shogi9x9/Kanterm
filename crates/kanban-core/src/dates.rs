use anyhow::{anyhow, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const MS_PER_DAY: i64 = 86_400_000;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Days since the Unix epoch for a civil (y, m, d) date.
/// Howard Hinnant's algorithm; avoids pulling in a date crate.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of `days_from_civil`.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parse "YYYY-MM-DD" into epoch milliseconds at UTC midnight.
pub fn parse_date(s: &str) -> Result<i64> {
    let parts: Vec<&str> = s.trim().split('-').collect();
    if parts.len() != 3 {
        return Err(anyhow!("date must be YYYY-MM-DD, got '{s}'"));
    }
    let y: i64 = parts[0].parse().map_err(|_| anyhow!("bad year in '{s}'"))?;
    let m: i64 = parts[1]
        .parse()
        .map_err(|_| anyhow!("bad month in '{s}'"))?;
    let d: i64 = parts[2].parse().map_err(|_| anyhow!("bad day in '{s}'"))?;
    if !(1..=12).contains(&m) {
        return Err(anyhow!("date out of range: '{s}'"));
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let dim = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if d < 1 || d > dim[(m - 1) as usize] {
        return Err(anyhow!("date out of range: '{s}'"));
    }
    Ok(days_from_civil(y, m, d) * MS_PER_DAY)
}

/// Format epoch milliseconds as "YYYY-MM-DD" (UTC).
pub fn format_date(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(MS_PER_DAY));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Epoch milliseconds at the start of today (UTC). A due date strictly before
/// this is overdue.
pub fn today_start_ms() -> i64 {
    now_ms().div_euclid(MS_PER_DAY) * MS_PER_DAY
}
