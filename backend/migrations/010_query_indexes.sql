-- 010_query_indexes.sql
CREATE INDEX IF NOT EXISTS idx_game_participants_user_id_game_id
    ON game_participants(user_id, game_id);

CREATE INDEX IF NOT EXISTS idx_games_finished_at_id
    ON games(finished_at DESC, id);

CREATE INDEX IF NOT EXISTS idx_user_stats_total_score_user_id
    ON user_stats(total_score DESC, user_id);
