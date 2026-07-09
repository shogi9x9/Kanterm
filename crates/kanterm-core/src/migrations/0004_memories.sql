-- Migration 0004: the memory log — decisions, learnings and context that
-- should survive across agent sessions. Memories link to cards by key TEXT
-- (no FK) so they outlive board/card deletion.

CREATE TABLE memories (
    id          TEXT PRIMARY KEY,
    key_text    TEXT NOT NULL UNIQUE,         -- e.g. M-12, one global sequence
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    kind        TEXT NOT NULL DEFAULT 'note', -- decision / learning / context / note
    card_key    TEXT,                         -- loose link, e.g. 'KB-12'
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    archived_at INTEGER
);

CREATE INDEX idx_memories_card ON memories(card_key);
CREATE INDEX idx_memories_updated ON memories(updated_at);

-- Generic named counters; 'memory_seq' allocates memory keys the same way
-- boards.card_seq allocates card keys.
CREATE TABLE counters (
    name  TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);
INSERT INTO counters (name, value) VALUES ('memory_seq', 0);
