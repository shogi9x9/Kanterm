use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::labels::upsert_label;
use crate::CardPatch;

pub(in crate::cards) fn apply_label_changes(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
    ts: i64,
) -> Result<()> {
    if let Some(names) = &patch.add_labels {
        for name in names {
            let label_id = upsert_label(tx, name)?;
            tx.execute(
                "INSERT OR IGNORE INTO card_labels (card_id, label_id) VALUES (?1, ?2)",
                params![card_id, label_id],
            )?;
            tx.execute(
                "UPDATE labels SET last_used_at = ?1 WHERE id = ?2",
                params![ts, label_id],
            )?;
        }
    }
    if let Some(names) = &patch.remove_labels {
        for name in names {
            tx.execute(
                "DELETE FROM card_labels WHERE card_id = ?1 AND label_id =
                   (SELECT id FROM labels WHERE name = ?2)",
                params![card_id, name],
            )?;
        }
    }
    Ok(())
}
