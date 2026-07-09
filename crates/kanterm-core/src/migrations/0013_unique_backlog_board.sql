-- Migration 0013: Backlog is a single reserved board.
--
-- Older builds could create a separate board named "Backlog" after the
-- protected `backlog` board already existed, yielding slugs like `backlog-2`.
-- Keep the protected board and discard those duplicate Backlog boards.
--
-- The protected Backlog board also has a stricter shape than project boards:
-- it is an inbox/planning list with exactly one `Backlog` column.

DELETE FROM boards
 WHERE slug <> 'backlog'
   AND lower(trim(name)) = 'backlog';

UPDATE columns
   SET name = 'Backlog',
       sort_order = 0
 WHERE id = (
       SELECT c.id
         FROM columns c
         JOIN boards b ON b.id = c.board_id
        WHERE b.slug = 'backlog'
        ORDER BY c.sort_order, c.created_at
        LIMIT 1
   )
   AND NOT EXISTS (
       SELECT 1
         FROM columns c
         JOIN boards b ON b.id = c.board_id
        WHERE b.slug = 'backlog'
          AND c.name = 'Backlog'
   );

INSERT INTO columns (id, board_id, name, sort_order, wip_limit, created_at)
SELECT lower(hex(randomblob(8))), b.id, 'Backlog', 0, NULL,
       CAST(strftime('%s', 'now') AS INTEGER) * 1000
  FROM boards b
 WHERE b.slug = 'backlog'
   AND NOT EXISTS (
       SELECT 1 FROM columns c WHERE c.board_id = b.id
   );

UPDATE cards
   SET column_id = (
       SELECT c.id
         FROM columns c
         JOIN boards b ON b.id = c.board_id
        WHERE b.slug = 'backlog'
          AND c.name = 'Backlog'
        LIMIT 1
   )
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'backlog');

DELETE FROM columns
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'backlog')
   AND name <> 'Backlog';

UPDATE columns
   SET sort_order = 0
 WHERE board_id = (SELECT id FROM boards WHERE slug = 'backlog')
   AND name = 'Backlog';
