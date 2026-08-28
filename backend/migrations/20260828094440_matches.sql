-- Add migration script here
CREATE TABLE matches (
    id BLOB PRIMARY KEY,
    white_bot_id BLOB NOT NULL,
    black_bot_id BLOB NOT NULL,
    match_status VARCHAR(20) NOT NULL DEFAULT 'pending',
    is_ranked BOOLEAN NOT NULL DEFAULT 0,
    winner_color VARCHAR(10) NULL,
    win_reason VARCHAR(50) NULL,
    pgn TEXT,
    white_elo_change INTEGER NULL,
    black_elo_change INTEGER NULL,
    error_message TEXT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME NULL,
    FOREIGN KEY (white_bot_id) REFERENCES bots(id),
    FOREIGN KEY (black_bot_id) REFERENCES bots(id)
);
-- It has to have both bot_ids for each bot, and then somehow include both user_ids, match record, match result