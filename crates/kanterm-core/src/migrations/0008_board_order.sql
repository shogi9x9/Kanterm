-- Migration 0008: explicit board ordering for TUI/MCP board selectors.

ALTER TABLE boards ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

UPDATE boards
   SET sort_order = (
       SELECT COUNT(*)
         FROM boards b2
        WHERE b2.created_at < boards.created_at
           OR (b2.created_at = boards.created_at AND b2.id <= boards.id)
   ) - 1;

CREATE INDEX idx_boards_sort_order ON boards(sort_order);
