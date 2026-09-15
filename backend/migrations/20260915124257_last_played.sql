-- Add migration script here
ALTER TABLE bots ADD COLUMN last_played_at TIMESTAMP DEFAULT NULL;
