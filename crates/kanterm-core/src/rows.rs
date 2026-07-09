use crate::{Board, Card, Column, Memory};

pub(crate) const BOARD_COLUMNS: &str =
    "id, name, slug, key_prefix, card_seq, sort_order, archived_at, agent_context";

pub(crate) const CARD_COLUMNS: &str = "id, board_id, column_id, key_text, title, body, status, priority, assignee, due_date, next_action, blocked_reason, acceptance_criteria, handoff_note, last_verification, agent_weight, agent_effort, suggested_model, expected_tokens, human_intervention, claimed_by, claimed_at, lease_expires_at, position, created_at, updated_at, archived_at";

pub(crate) const CARD_SELECT_PREFIX_WHERE_BOARD: &str = "SELECT id, board_id, column_id, key_text, title, body, status, priority, assignee, due_date, next_action, blocked_reason, acceptance_criteria, handoff_note, last_verification, agent_weight, agent_effort, suggested_model, expected_tokens, human_intervention, claimed_by, claimed_at, lease_expires_at, position, created_at, updated_at, archived_at FROM cards WHERE board_id = ?1 AND archived_at IS NULL ORDER BY column_id, position";

pub(crate) const CARD_SELECT_BY_KEY: &str = "SELECT id, board_id, column_id, key_text, title, body, status, priority, assignee, due_date, next_action, blocked_reason, acceptance_criteria, handoff_note, last_verification, agent_weight, agent_effort, suggested_model, expected_tokens, human_intervention, claimed_by, claimed_at, lease_expires_at, position, created_at, updated_at, archived_at FROM cards WHERE board_id = ?1 AND key_text = ?2";
pub(crate) const CARD_SELECT_BY_ID: &str = "SELECT id, board_id, column_id, key_text, title, body, status, priority, assignee, due_date, next_action, blocked_reason, acceptance_criteria, handoff_note, last_verification, agent_weight, agent_effort, suggested_model, expected_tokens, human_intervention, claimed_by, claimed_at, lease_expires_at, position, created_at, updated_at, archived_at FROM cards WHERE id = ?1";

pub(crate) const MEMORY_COLUMNS: &str =
    "id, key_text, title, body, kind, card_key, created_at, updated_at, archived_at, last_recalled_at, recall_count";

pub(crate) fn row_to_memory(r: &rusqlite::Row) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: r.get(0)?,
        key: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        kind: r.get(4)?,
        card_key: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
        archived_at: r.get(8)?,
        last_recalled_at: r.get(9)?,
        recall_count: r.get(10)?,
    })
}

pub(crate) fn row_to_board(r: &rusqlite::Row) -> rusqlite::Result<Board> {
    Ok(Board {
        id: r.get(0)?,
        name: r.get(1)?,
        slug: r.get(2)?,
        key_prefix: r.get(3)?,
        card_seq: r.get(4)?,
        sort_order: r.get(5)?,
        archived_at: r.get(6)?,
        agent_context: r.get(7)?,
    })
}

pub(crate) fn row_to_column(r: &rusqlite::Row) -> rusqlite::Result<Column> {
    Ok(Column {
        id: r.get(0)?,
        board_id: r.get(1)?,
        name: r.get(2)?,
        sort_order: r.get(3)?,
        wip_limit: r.get(4)?,
    })
}

pub(crate) fn row_to_card(r: &rusqlite::Row) -> rusqlite::Result<Card> {
    Ok(Card {
        id: r.get(0)?,
        board_id: r.get(1)?,
        column_id: r.get(2)?,
        key: r.get(3)?,
        title: r.get(4)?,
        body: r.get(5)?,
        agent_state: r.get(6)?,
        priority: r.get(7)?,
        assignee: r.get(8)?,
        due_date: r.get(9)?,
        next_action: r.get(10)?,
        blocked_reason: r.get(11)?,
        acceptance_criteria: r.get(12)?,
        handoff_note: r.get(13)?,
        last_verification: r.get(14)?,
        agent_weight: r.get(15)?,
        agent_effort: r.get(16)?,
        suggested_model: r.get(17)?,
        expected_tokens: r.get(18)?,
        human_intervention: r.get(19)?,
        claimed_by: r.get(20)?,
        claimed_at: r.get(21)?,
        lease_expires_at: r.get(22)?,
        position: r.get(23)?,
        created_at: r.get(24)?,
        updated_at: r.get(25)?,
        archived_at: r.get(26)?,
    })
}
