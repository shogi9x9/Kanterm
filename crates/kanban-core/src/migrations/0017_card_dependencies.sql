CREATE TABLE card_dependencies (
    board_id            TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    downstream_card_id  TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    upstream_card_id    TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    created_at          INTEGER NOT NULL,
    PRIMARY KEY (downstream_card_id, upstream_card_id),
    CHECK (downstream_card_id <> upstream_card_id)
);

CREATE INDEX idx_card_dependencies_board_downstream
    ON card_dependencies(board_id, downstream_card_id);

CREATE INDEX idx_card_dependencies_board_upstream
    ON card_dependencies(board_id, upstream_card_id);
