-- Migration 0001: initial schema (user_version 0 -> 1).

CREATE TABLE boards (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    key_prefix  TEXT NOT NULL DEFAULT 'KB',
    card_seq    INTEGER NOT NULL DEFAULT 0,   -- monotonic counter for card keys
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE columns (
    id          TEXT PRIMARY KEY,
    board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    sort_order  INTEGER NOT NULL,
    wip_limit   INTEGER,
    created_at  INTEGER NOT NULL,
    UNIQUE(board_id, name)
);

CREATE TABLE cards (
    id          TEXT PRIMARY KEY,
    board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    column_id   TEXT NOT NULL REFERENCES columns(id) ON DELETE CASCADE,
    key_text    TEXT NOT NULL,                 -- e.g. KB-12, shown to humans & agents
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'open',
    priority    INTEGER NOT NULL DEFAULT 1,    -- 0 low, 1 normal, 2 high
    assignee    TEXT,
    due_date    INTEGER,
    position    REAL NOT NULL,                 -- fractional index within a column
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,              -- optimistic-concurrency anchor
    archived_at INTEGER,
    UNIQUE(board_id, key_text)
);

CREATE TABLE labels (
    id     TEXT PRIMARY KEY,
    name   TEXT NOT NULL UNIQUE,
    color  TEXT NOT NULL DEFAULT '#66cc88'
);

CREATE TABLE card_labels (
    card_id  TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, label_id)
);

CREATE TABLE activity_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id      TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    actor        TEXT NOT NULL DEFAULT 'local', -- 'tui', 'agent', etc.
    action       TEXT NOT NULL,                 -- create / update / move
    payload_json TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);

CREATE TABLE ui_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX idx_cards_board_col ON cards(board_id, column_id, position);
CREATE INDEX idx_cards_key ON cards(board_id, key_text);
CREATE INDEX idx_activity_card ON activity_logs(card_id, created_at);
