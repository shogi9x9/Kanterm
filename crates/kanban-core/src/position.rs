use anyhow::Result;
use rusqlite::params;

/// Append position: max(position) in the column + 1.0, or 1.0 if empty.
/// Fractional positions keep moves O(1) and friendly to concurrent writers.
pub(crate) fn next_position(tx: &rusqlite::Transaction, column_id: &str) -> Result<f64> {
    let max: Option<f64> = tx.query_row(
        "SELECT MAX(position) FROM cards WHERE column_id = ?1 AND archived_at IS NULL",
        params![column_id],
        |r| r.get(0),
    )?;
    Ok(max.unwrap_or(0.0) + 1.0)
}
