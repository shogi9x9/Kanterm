-- Migration 0019: durable agent-to-agent handoff queue.

CREATE TABLE IF NOT EXISTS agent_handoffs (
    id TEXT PRIMARY KEY,
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,
    board_id TEXT REFERENCES boards(id) ON DELETE SET NULL,
    card_key TEXT,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    claimed_by TEXT,
    claimed_at INTEGER,
    lease_expires_at INTEGER,
    completed_at INTEGER,
    failed_at INTEGER,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_handoffs_inbox
    ON agent_handoffs(to_agent, status, created_at);

CREATE INDEX IF NOT EXISTS idx_agent_handoffs_claims
    ON agent_handoffs(status, lease_expires_at);
