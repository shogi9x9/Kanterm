-- Migration 0002: track when a label was last attached to a card, so the TUI
-- can hide labels that have gone stale (user_version 1 -> 2).

ALTER TABLE labels ADD COLUMN last_used_at INTEGER;

-- Backfill existing labels as "used now" so they aren't immediately hidden.
UPDATE labels SET last_used_at = strftime('%s', 'now') * 1000 WHERE last_used_at IS NULL;

CREATE INDEX idx_labels_last_used ON labels(last_used_at);
