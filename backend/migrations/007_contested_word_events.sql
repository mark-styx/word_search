CREATE TABLE IF NOT EXISTS contested_word_events (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id),
    word TEXT NOT NULL,
    winning_user_id TEXT NOT NULL REFERENCES users(id),
    losing_user_id TEXT NOT NULL REFERENCES users(id),
    winning_found_at TEXT NOT NULL,
    losing_attempted_at TEXT NOT NULL,
    delta_ms INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 1,
    UNIQUE(game_id, word, winning_user_id, losing_user_id)
);

CREATE INDEX IF NOT EXISTS idx_contested_word_events_winner
    ON contested_word_events(winning_user_id, delta_ms ASC, losing_attempted_at DESC);

CREATE INDEX IF NOT EXISTS idx_contested_word_events_loser
    ON contested_word_events(losing_user_id, delta_ms ASC, losing_attempted_at DESC);
