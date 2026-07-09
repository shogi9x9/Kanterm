-- Migration 0006: structured fields for AI-agent handoff and resumption.

ALTER TABLE cards ADD COLUMN next_action TEXT;
ALTER TABLE cards ADD COLUMN blocked_reason TEXT;
ALTER TABLE cards ADD COLUMN acceptance_criteria TEXT;
