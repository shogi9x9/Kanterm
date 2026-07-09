-- Migration 0011: rename the visible default board from Main to Backlog.
--
-- Keep the `main` slug stable for MCP/API compatibility; only the user-facing
-- board name changes.

UPDATE boards
   SET name = 'Backlog',
       updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
 WHERE slug = 'main'
   AND name = 'Main';
