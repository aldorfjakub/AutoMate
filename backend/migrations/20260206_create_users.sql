CREATE TABLE users (
    id BLOB PRIMARY KEY,
    email TEXT UNIQUE,
    display_name TEXT,
    avatar_url TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE oauth_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id BLOB NOT NULL,
    provider TEXT NOT NULL,
    provider_user_id TEXT NOT NULL, 
    access_token TEXT,
    refresh_token TEXT,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(provider, provider_user_id)
);

CREATE TABLE sessions (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    expires_at DATETIME NOT NULL,
    user_agent TEXT,
    ip_address TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);