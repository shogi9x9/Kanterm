use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};
use std::collections::HashSet;

use crate::activity::log_activity_payload;
use crate::{now_ms, Card};

use super::{cycle, read};

pub(super) fn set_card_dependencies_tx(
    tx: &Transaction<'_>,
    board_id: &str,
    key: &str,
    upstream_keys: &[String],
    actor: &str,
) -> Result<String> {
    let downstream = read::load_card_by_key_tx(tx, board_id, key)?
        .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
    let upstream = load_upstream_cards(tx, board_id, &downstream, upstream_keys)?;
    cycle::validate_acyclic(tx, board_id, &downstream.id, &upstream)?;

    tx.execute(
        "DELETE FROM card_dependencies WHERE board_id = ?1 AND downstream_card_id = ?2",
        params![board_id, downstream.id],
    )?;
    let ts = now_ms();
    for card in &upstream {
        tx.execute(
            "INSERT INTO card_dependencies
               (board_id, downstream_card_id, upstream_card_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![board_id, downstream.id, card.id, ts],
        )?;
    }
    log_activity_payload(
        tx,
        &downstream.id,
        actor,
        "dependencies",
        serde_json::json!({
            "detail": format!("{} depends on {}", downstream.key, upstream.iter().map(|c| c.key.as_str()).collect::<Vec<_>>().join(", ")),
        }),
    )?;
    Ok(downstream.id)
}

fn load_upstream_cards(
    tx: &Transaction<'_>,
    board_id: &str,
    downstream: &Card,
    upstream_keys: &[String],
) -> Result<Vec<Card>> {
    let mut seen = HashSet::new();
    let mut cards = Vec::new();
    for key in upstream_keys {
        if !seen.insert(key.clone()) {
            continue;
        }
        if key == &downstream.key {
            return Err(anyhow!("card '{}' cannot depend on itself", downstream.key));
        }
        let upstream = read::load_card_by_key_tx(tx, board_id, key)?
            .ok_or_else(|| anyhow!("dependency references missing card '{key}'"))?;
        cards.push(upstream);
    }
    Ok(cards)
}
