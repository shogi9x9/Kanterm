-- Migration 0016: metadata that helps agents decide whether/how to execute a card.
ALTER TABLE cards ADD COLUMN agent_weight INTEGER;
ALTER TABLE cards ADD COLUMN agent_effort TEXT;
ALTER TABLE cards ADD COLUMN suggested_model TEXT;
ALTER TABLE cards ADD COLUMN expected_tokens INTEGER;
ALTER TABLE cards ADD COLUMN human_intervention TEXT;
