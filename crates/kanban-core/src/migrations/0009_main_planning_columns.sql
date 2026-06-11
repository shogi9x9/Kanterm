-- Migration 0009: Backlog is a task lane, not a board.

UPDATE columns
   SET name = 'Today'
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'main')
   AND name = '今日中にやる'
   AND NOT EXISTS (
       SELECT 1 FROM columns existing
        WHERE existing.board_id = columns.board_id
          AND existing.name = 'Today'
   );

UPDATE columns
   SET name = 'This week'
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'main')
   AND name = '今週中にやる'
   AND NOT EXISTS (
       SELECT 1 FROM columns existing
        WHERE existing.board_id = columns.board_id
          AND existing.name = 'This week'
   );

UPDATE columns
   SET name = 'This month'
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'main')
   AND name = 'いつかやる'
   AND NOT EXISTS (
       SELECT 1 FROM columns existing
        WHERE existing.board_id = columns.board_id
          AND existing.name = 'This month'
   );

UPDATE columns
   SET sort_order = sort_order + 1
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'main')
   AND NOT EXISTS (
       SELECT 1 FROM columns existing
        WHERE existing.board_id = columns.board_id
          AND existing.name = 'Backlog'
   );

INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
SELECT lower(hex(randomblob(8))), b.id, 'Backlog', 0, NULL,
       CAST(strftime('%s', 'now') AS INTEGER) * 1000
  FROM boards b
 WHERE b.slug = 'main'
   AND NOT EXISTS (
       SELECT 1 FROM columns existing
        WHERE existing.board_id = b.id
          AND existing.name = 'Backlog'
   );

UPDATE cards
   SET board_id = (SELECT id FROM boards WHERE slug = 'main'),
       column_id = (
           SELECT c.id
             FROM columns c
             JOIN boards b ON b.id = c.board_id
            WHERE b.slug = 'main'
              AND c.name = 'Backlog'
            LIMIT 1
       )
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'backlog')
   AND NOT EXISTS (
       SELECT 1 FROM cards existing
        WHERE existing.board_id = (SELECT id FROM boards WHERE slug = 'main')
          AND existing.key_text = cards.key_text
   );

DELETE FROM boards
 WHERE slug = 'backlog'
   AND NOT EXISTS (
       SELECT 1 FROM cards
        WHERE cards.board_id = boards.id
   );
