use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::position::next_position;
use crate::CardPatch;

pub(in crate::cards) fn apply_column_move(
    tx: &Transaction<'_>,
    board_id: &str,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(column_name) = &patch.column {
        let column_id: String = tx
            .query_row(
                "SELECT id FROM columns WHERE board_id = ?1 AND name = ?2",
                params![board_id, column_name],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("no column named '{column_name}'"))?;
        let position = next_position(tx, &column_id)?;
        tx.execute(
            "UPDATE cards SET column_id = ?1, position = ?2 WHERE id = ?3",
            params![column_id, position, card_id],
        )?;
    }
    Ok(())
}

pub(in crate::cards) fn apply_board_rehome(
    tx: &Transaction<'_>,
    source_board_id: &str,
    target_board_id: &mut String,
    card_id: &str,
    old_key: &str,
    patch: &CardPatch,
) -> Result<Option<BoardMoveActivity>> {
    let Some(dest_board_id) = &patch.move_to_board else {
        return Ok(None);
    };
    if dest_board_id == source_board_id {
        return Ok(None);
    }

    let (source_name, source_slug): (String, String) = tx
        .query_row(
            "SELECT name, slug FROM boards WHERE id = ?1",
            params![source_board_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no board '{source_board_id}'"))?;

    let (dest_name, dest_slug, prefix, seq): (String, String, String, i64) = tx
        .query_row(
            "SELECT name, slug, key_prefix, card_seq FROM boards WHERE id = ?1",
            params![dest_board_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no board '{dest_board_id}'"))?;

    let new_key = format!("{}-{}", prefix, seq + 1);
    tx.execute(
        "UPDATE boards SET card_seq = ?1 WHERE id = ?2",
        params![seq + 1, dest_board_id],
    )?;

    let dest_col: String = tx
        .query_row(
            "SELECT id FROM columns WHERE board_id = ?1 ORDER BY sort_order LIMIT 1",
            params![dest_board_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("destination board has no columns"))?;

    tx.execute(
        "UPDATE cards SET board_id = ?1, column_id = ?2, position = ?3, key_text = ?4 WHERE id = ?5",
        params![
            dest_board_id,
            dest_col,
            next_position(tx, &dest_col)?,
            new_key,
            card_id
        ],
    )?;
    *target_board_id = dest_board_id.to_string();
    Ok(Some(BoardMoveActivity {
        old_key: old_key.to_string(),
        new_key,
        source_board_id: source_board_id.to_string(),
        source_board_name: source_name,
        source_board_slug: source_slug,
        destination_board_id: dest_board_id.to_string(),
        destination_board_name: dest_name,
        destination_board_slug: dest_slug,
    }))
}

#[derive(Debug)]
pub(in crate::cards) struct BoardMoveActivity {
    old_key: String,
    new_key: String,
    source_board_id: String,
    source_board_name: String,
    source_board_slug: String,
    destination_board_id: String,
    destination_board_name: String,
    destination_board_slug: String,
}

impl BoardMoveActivity {
    pub(in crate::cards) fn into_payload(self) -> serde_json::Value {
        serde_json::json!({
            "old_key": self.old_key,
            "new_key": self.new_key,
            "source_board": {
                "id": self.source_board_id,
                "name": self.source_board_name,
                "slug": self.source_board_slug,
            },
            "destination_board": {
                "id": self.destination_board_id,
                "name": self.destination_board_name,
                "slug": self.destination_board_slug,
            },
        })
    }
}
