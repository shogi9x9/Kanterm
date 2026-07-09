-- Boards become archivable instead of delete-only. NULL = active.
ALTER TABLE boards ADD COLUMN archived_at INTEGER;
