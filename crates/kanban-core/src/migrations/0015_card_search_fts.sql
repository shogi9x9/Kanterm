-- Migration 0015: FTS5 search index for active cards.

CREATE VIRTUAL TABLE IF NOT EXISTS card_search USING fts5(
    card_id UNINDEXED,
    board_id UNINDEXED,
    title,
    body,
    labels,
    agent_fields,
    tokenize = 'unicode61'
);

INSERT INTO card_search (card_id, board_id, title, body, labels, agent_fields)
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
        COALESCE(c.last_verification, '')
    )
  FROM cards c
 WHERE c.archived_at IS NULL;
