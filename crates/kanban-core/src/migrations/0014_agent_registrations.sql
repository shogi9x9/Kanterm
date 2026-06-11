-- Migration 0014: self-registered agent identities for verifiable claims and
-- future inbox routing.

CREATE TABLE IF NOT EXISTS agent_registrations (
    id                TEXT PRIMARY KEY,
    requested_name    TEXT NOT NULL,
    assigned_identity TEXT NOT NULL UNIQUE,
    token_hash        TEXT NOT NULL,
    fingerprint_json  TEXT,
    registered_at     INTEGER NOT NULL,
    last_seen_at      INTEGER NOT NULL,
    expires_at        INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_registrations_requested
    ON agent_registrations(requested_name, expires_at);
