-- 001_users.sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    recovery_pin_hash TEXT,
    is_guest INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    visual_theme TEXT NOT NULL DEFAULT 'default',
    preferred_word_theme TEXT,
    highlight_color TEXT NOT NULL DEFAULT '#3b82f6'
);
