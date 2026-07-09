-- Migration 0005: memory recall tracking and monthly retention.
-- Agent recalls update these fields; read-only TUI browsing does not.

ALTER TABLE memories ADD COLUMN last_recalled_at INTEGER;
ALTER TABLE memories ADD COLUMN recall_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_memories_recall ON memories(recall_count, created_at);

INSERT INTO counters (name, value) VALUES ('memory_gc_last_run', 0);
