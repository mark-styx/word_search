-- 004_found_words.sql
CREATE TABLE IF NOT EXISTS found_words (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    word TEXT NOT NULL,
    found_at TEXT NOT NULL DEFAULT (datetime('now'))
);
