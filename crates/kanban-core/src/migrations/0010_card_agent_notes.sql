-- Migration 0010: agent-only handoff and verification metadata.

ALTER TABLE cards ADD COLUMN handoff_note TEXT;
ALTER TABLE cards ADD COLUMN last_verification TEXT;
