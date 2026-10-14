DELETE FROM found_words
WHERE rowid NOT IN (
    SELECT MIN(rowid)
    FROM found_words
    GROUP BY game_id, word
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_found_words_game_word_unique
    ON found_words(game_id, word);

CREATE INDEX IF NOT EXISTS idx_found_words_user_found_at
    ON found_words(user_id, found_at DESC);

CREATE INDEX IF NOT EXISTS idx_found_words_game_user_found_at
    ON found_words(game_id, user_id, found_at ASC);
