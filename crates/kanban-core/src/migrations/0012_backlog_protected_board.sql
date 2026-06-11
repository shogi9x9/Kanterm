-- Migration 0012: make Backlog the only protected default board.
--
-- This intentionally drops a leftover `main` board when a real `backlog` board
-- already exists. If `main` is the only default board, keep its data by moving
-- the slug to `backlog`.

UPDATE boards
   SET slug = 'backlog',
       name = 'Backlog',
       key_prefix = 'KB',
       archived_at = NULL,
       updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
 WHERE slug = 'main'
   AND NOT EXISTS (SELECT 1 FROM boards WHERE slug = 'backlog');

DELETE FROM boards
 WHERE slug = 'main'
   AND EXISTS (SELECT 1 FROM boards WHERE slug = 'backlog');

UPDATE boards
   SET name = 'Backlog',
       key_prefix = 'KB',
       archived_at = NULL,
       updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
 WHERE slug = 'backlog';

UPDATE ui_state
   SET value = 'backlog'
 WHERE key = 'tui.board'
   AND value = 'main';
