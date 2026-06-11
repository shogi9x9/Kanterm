-- Migration 0018: board-level execution guidance for agents.

ALTER TABLE boards ADD COLUMN agent_context TEXT;
