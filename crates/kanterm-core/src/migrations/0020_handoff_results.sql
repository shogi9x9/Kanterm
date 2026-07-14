-- Migration 0020: persist successful handoff results for sender-side retrieval.

ALTER TABLE agent_handoffs ADD COLUMN result_text TEXT;
