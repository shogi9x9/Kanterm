use std::collections::HashSet;

use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::rows::row_to_card;
use crate::text::{like_escape, trimmed_optional};
use crate::{Card, Store};

const SEARCH_CARD_COLUMNS: &str = "c.id, c.board_id, c.column_id, c.key_text, c.title, c.body, c.status, c.priority, c.assignee, c.due_date, c.next_action, c.blocked_reason, c.acceptance_criteria, c.handoff_note, c.last_verification, c.agent_weight, c.agent_effort, c.suggested_model, c.expected_tokens, c.human_intervention, c.claimed_by, c.claimed_at, c.lease_expires_at, c.position, c.created_at, c.updated_at, c.archived_at";

impl Store {
    pub fn search_cards(&self, board_id: &str, query: &str) -> Result<Vec<Card>> {
        let Some(query) = trimmed_optional(query) else {
            return self.cards(board_id);
        };

        let mut cards = Vec::new();
        let mut seen = HashSet::new();
        if let Some(fts_query) = fts_query(query) {
            for card in self.search_cards_fts(board_id, &fts_query)? {
                seen.insert(card.id.clone());
                cards.push(card);
            }
        }

        // Keep the old literal-substring behavior as a complement for short
        // fragments, punctuation-heavy queries and exact label/body snippets.
        for card in self.search_cards_like(board_id, query)? {
            if seen.insert(card.id.clone()) {
                cards.push(card);
            }
        }
        cards.sort_by(|a, b| {
            a.column_id
                .cmp(&b.column_id)
                .then_with(|| a.position.total_cmp(&b.position))
        });
        Ok(cards)
    }

    fn search_cards_fts(&self, board_id: &str, query: &str) -> Result<Vec<Card>> {
        let sql = format!(
            "SELECT {SEARCH_CARD_COLUMNS}
               FROM card_search s
               JOIN cards c ON c.id = s.card_id
              WHERE s.board_id = ?1
                AND card_search MATCH ?2
                AND c.archived_at IS NULL
              ORDER BY c.column_id, c.position"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![board_id, query], row_to_card)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn search_cards_like(&self, board_id: &str, query: &str) -> Result<Vec<Card>> {
        let sql = format!(
            "SELECT {SEARCH_CARD_COLUMNS}
               FROM cards c
              WHERE c.board_id = ?1
                AND c.archived_at IS NULL
                AND (
                    c.title LIKE ?2 ESCAPE '\\'
                    OR c.body LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.next_action, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.blocked_reason, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.acceptance_criteria, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.handoff_note, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.last_verification, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.agent_effort, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.suggested_model, '') LIKE ?2 ESCAPE '\\'
                    OR IFNULL(c.human_intervention, '') LIKE ?2 ESCAPE '\\'
                    OR EXISTS (
                        SELECT 1
                          FROM card_labels cl
                          JOIN labels l ON l.id = cl.label_id
                         WHERE cl.card_id = c.id
                           AND l.name LIKE ?2 ESCAPE '\\'
                    )
                )
              ORDER BY c.column_id, c.position"
        );
        let pattern = format!("%{}%", like_escape(query));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![board_id, pattern], row_to_card)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

pub(crate) fn sync_card_search_row(tx: &Transaction<'_>, card_id: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM card_search WHERE card_id = ?1",
        params![card_id],
    )?;
    tx.execute(
        "INSERT INTO card_search (card_id, board_id, title, body, labels, agent_fields)
         SELECT
             c.id,
             c.board_id,
             c.title,
             c.body,
             COALESCE((
                 SELECT group_concat(l.name, ' ')
                   FROM card_labels cl
                   JOIN labels l ON l.id = cl.label_id
                  WHERE cl.card_id = c.id
             ), ''),
             trim(
                 COALESCE(c.next_action, '') || ' ' ||
                 COALESCE(c.blocked_reason, '') || ' ' ||
                 COALESCE(c.acceptance_criteria, '') || ' ' ||
                 COALESCE(c.handoff_note, '') || ' ' ||
                 COALESCE(c.last_verification, '') || ' ' ||
                 COALESCE(c.agent_effort, '') || ' ' ||
                 COALESCE(c.suggested_model, '') || ' ' ||
                 COALESCE(c.human_intervention, '')
             )
           FROM cards c
          WHERE c.id = ?1
            AND c.archived_at IS NULL",
        params![card_id],
    )?;
    Ok(())
}

fn fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter_map(|term| {
            let term = term.trim_matches(|c: char| !c.is_alphanumeric());
            trimmed_optional(term).map(quote_fts_term)
        })
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

fn quote_fts_term(term: &str) -> String {
    format!("\"{}\"", term.replace('"', "\"\""))
}
