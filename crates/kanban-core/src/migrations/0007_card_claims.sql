-- Migration 0007: optional card leases for agent collision avoidance.

ALTER TABLE cards ADD COLUMN claimed_by TEXT;
ALTER TABLE cards ADD COLUMN claimed_at INTEGER;
ALTER TABLE cards ADD COLUMN lease_expires_at INTEGER;

CREATE INDEX idx_cards_lease ON cards(lease_expires_at);
