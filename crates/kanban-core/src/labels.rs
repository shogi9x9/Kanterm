use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use crate::id::new_id;
use crate::{now_ms, Label, Store};

/// Get-or-create a label by name, returning its id. Colour is derived
/// deterministically from the name so the same tag always looks the same.
pub(crate) fn upsert_label(tx: &rusqlite::Transaction, name: &str) -> Result<String> {
    if let Some(id) = tx
        .query_row(
            "SELECT id FROM labels WHERE name = ?1",
            params![name],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    let palette = [
        "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2",
    ];
    let idx = name.bytes().fold(0u32, |a, b| a.wrapping_add(b as u32)) as usize % palette.len();
    let id = new_id();
    tx.execute(
        "INSERT INTO labels (id, name, color, last_used_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, palette[idx], now_ms()],
    )?;
    Ok(id)
}

impl Store {
    pub fn list_labels(&self) -> Result<Vec<Label>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, color FROM labels ORDER BY name")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Label {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Labels attached to a card within the last `within_ms`, most-recent first.
    /// Used to offer reusable label suggestions while hiding stale ones.
    pub fn recent_labels(&self, within_ms: i64) -> Result<Vec<Label>> {
        if within_ms <= 0 {
            return Ok(Vec::new());
        }
        let cutoff = now_ms() - within_ms;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color FROM labels
             WHERE last_used_at IS NOT NULL AND last_used_at >= ?1
             ORDER BY last_used_at DESC",
        )?;
        let rows = stmt
            .query_map(params![cutoff], |r| {
                Ok(Label {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// All labels for every card on a board, keyed by card id. One query so the
    /// TUI/board renderer doesn't fan out per card.
    pub fn labels_by_card(&self, board_id: &str) -> Result<HashMap<String, Vec<Label>>> {
        let mut stmt = self.conn.prepare(
            "SELECT cl.card_id, l.id, l.name, l.color
               FROM card_labels cl
               JOIN labels l ON l.id = cl.label_id
               JOIN cards c ON c.id = cl.card_id
              WHERE c.board_id = ?1
              ORDER BY l.name",
        )?;
        let mut map: HashMap<String, Vec<Label>> = HashMap::new();
        let rows = stmt.query_map(params![board_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Label {
                    id: r.get(1)?,
                    name: r.get(2)?,
                    color: r.get(3)?,
                },
            ))
        })?;
        for row in rows {
            let (card_id, label) = row?;
            map.entry(card_id).or_default().push(label);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::{CardPatch, Store};

    #[test]
    fn list_labels_is_sorted_by_name() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        store
            .create_card(&board.id, None, "task", "", "test")
            .unwrap();
        store
            .update_card(
                &board.id,
                "KB-1",
                &CardPatch {
                    add_labels: Some(vec!["zeta".into(), "alpha".into()]),
                    ..Default::default()
                },
                "test",
            )
            .unwrap();

        let names: Vec<String> = store
            .list_labels()
            .unwrap()
            .into_iter()
            .map(|label| label.name)
            .collect();
        assert_eq!(names, vec!["alpha", "zeta"]);
    }
}
