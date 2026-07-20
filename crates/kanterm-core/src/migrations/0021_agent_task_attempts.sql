-- Migration 0021: retain the exact work packet used by every automated attempt.
CREATE TABLE agent_task_attempts (
    id TEXT PRIMARY KEY,
    handoff_id TEXT NOT NULL REFERENCES agent_handoffs(id) ON DELETE CASCADE,
    attempt_no INTEGER NOT NULL,
    target_name TEXT NOT NULL,
    packet_version TEXT NOT NULL,
    packet_profile TEXT NOT NULL,
    packet_sha256 TEXT NOT NULL,
    packet_text TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'agent_succeeded', 'agent_failed')),
    agent_output TEXT,
    error_text TEXT,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    UNIQUE (handoff_id, attempt_no)
);

CREATE INDEX idx_agent_task_attempts_handoff
    ON agent_task_attempts(handoff_id, attempt_no);
