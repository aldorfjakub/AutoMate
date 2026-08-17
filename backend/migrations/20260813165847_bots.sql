-- Add migration script here
CREATE TABLE bots (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    source_code TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 0,
    is_valid BOOLEAN NOT NULL DEFAULT 0,
    is_public BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);