-- 002_games.sql
CREATE TABLE IF NOT EXISTS games (
    id TEXT PRIMARY KEY,
    theme TEXT NOT NULL,
    difficulty TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    time_limit_secs INTEGER,
    is_solo BOOLEAN NOT NULL DEFAULT 0,
    max_players INTEGER NOT NULL DEFAULT 8
);
