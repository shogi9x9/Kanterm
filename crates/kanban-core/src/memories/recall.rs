use anyhow::Result;
use rusqlite::types::ToSql;
use rusqlite::{params, TransactionBehavior};

use crate::rows::{row_to_memory, MEMORY_COLUMNS};
use crate::text::like_escape;
use crate::{now_ms, Memory, Store};

pub(super) fn mark_memories_recalled<'a, I>(store: &mut Store, keys: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let keys: Vec<&str> = keys.into_iter().filter(|k| !k.trim().is_empty()).collect();
    if keys.is_empty() {
        return Ok(());
    }
    store.assert_writable()?;
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let ts = now_ms();
    for key in keys {
        tx.execute(
            "UPDATE memories
                SET last_recalled_at = ?1,
                    recall_count = recall_count + 1
              WHERE key_text = ?2",
            params![ts, key],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub(super) fn recall_memories(
    conn: &rusqlite::Connection,
    query: Option<&str>,
    card_key: Option<&str>,
    kind: Option<&str>,
    limit: usize,
    include_archived: bool,
) -> Result<Vec<Memory>> {
    let (sql, args) = build_recall_query(query, card_key, kind, limit, include_archived);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(args.iter().map(|a| a.as_ref())),
            row_to_memory,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn build_recall_query(
    query: Option<&str>,
    card_key: Option<&str>,
    kind: Option<&str>,
    limit: usize,
    include_archived: bool,
) -> (String, Vec<Box<dyn ToSql>>) {
    let mut sql = format!(
        "SELECT {MEMORY_COLUMNS} FROM memories WHERE 1=1{}",
        if include_archived {
            ""
        } else {
            " AND archived_at IS NULL"
        }
    );
    let mut args: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        sql.push_str(&format!(
            " AND (title LIKE ?{n} ESCAPE '\\' OR body LIKE ?{n} ESCAPE '\\' \
               OR IFNULL(card_key,'') LIKE ?{n} ESCAPE '\\')",
            n = args.len() + 1
        ));
        args.push(Box::new(format!("%{}%", like_escape(q))));
    }
    if let Some(ck) = card_key.map(str::trim).filter(|c| !c.is_empty()) {
        sql.push_str(&format!(" AND card_key = ?{}", args.len() + 1));
        args.push(Box::new(ck.to_string()));
    }
    if let Some(k) = kind.map(str::trim).filter(|k| !k.is_empty()) {
        sql.push_str(&format!(" AND kind = ?{}", args.len() + 1));
        args.push(Box::new(k.to_string()));
    }
    sql.push_str(&format!(
        " ORDER BY updated_at DESC LIMIT ?{}",
        args.len() + 1
    ));
    args.push(Box::new(limit as i64));
    (sql, args)
}
