-- 003_game_participants.sql
CREATE TABLE IF NOT EXISTS game_participants (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    score INTEGER NOT NULL DEFAULT 0,
    words_found INTEGER NOT NULL DEFAULT 0,
    rank_position INTEGER,
    UNIQUE(game_id, user_id)
);
