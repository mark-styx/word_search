use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

pub type DbPool = SqlitePool;

pub async fn create_pool(database_path: &str) -> Result<DbPool, sqlx::Error> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(database_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let connection_string = format!("sqlite:{}?mode=rwc", database_path);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&connection_string)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), Box<dyn std::error::Error>> {
    // Run migrations manually from SQL files
    let migration_sql = include_str!("../../migrations/001_users.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/002_games.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/003_game_participants.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/004_found_words.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/005_user_stats.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    Ok(())
}

// ── User operations ─────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: String,
    pub visual_theme: String,
    pub preferred_word_theme: Option<String>,
}

pub async fn create_user(
    pool: &DbPool,
    id: &Uuid,
    username: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash) VALUES (?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_by_username(
    pool: &DbPool,
    username: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, username, password_hash, created_at, visual_theme, preferred_word_theme FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, username, password_hash, created_at, visual_theme, preferred_word_theme FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_user_theme(
    pool: &DbPool,
    user_id: &str,
    visual_theme: &str,
    preferred_word_theme: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET visual_theme = ?, preferred_word_theme = ? WHERE id = ?",
    )
    .bind(visual_theme)
    .bind(preferred_word_theme)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Game operations ─────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct GameRow {
    pub id: String,
    pub theme: String,
    pub difficulty: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub time_limit_secs: Option<i64>,
    pub is_solo: bool,
    pub max_players: i64,
}

pub async fn insert_game(
    pool: &DbPool,
    id: &Uuid,
    theme: &str,
    difficulty: &str,
    status: &str,
    time_limit_secs: Option<i64>,
    is_solo: bool,
    max_players: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO games (id, theme, difficulty, status, time_limit_secs, is_solo, max_players) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(theme)
    .bind(difficulty)
    .bind(status)
    .bind(time_limit_secs)
    .bind(is_solo)
    .bind(max_players)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_game_status(
    pool: &DbPool,
    id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let finished = if status == "finished" {
        Some(now)
    } else {
        None
    };
    sqlx::query(
        "UPDATE games SET status = ?, finished_at = COALESCE(?, finished_at) WHERE id = ?",
    )
    .bind(status)
    .bind(finished)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_game_started(
    pool: &DbPool,
    id: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE games SET started_at = ?, status = 'in_progress' WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Participant operations ──────────────────────────────────────────────

pub async fn insert_participant(
    pool: &DbPool,
    id: &Uuid,
    game_id: &str,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_participants (id, game_id, user_id) VALUES (?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(game_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_participant_stats(
    pool: &DbPool,
    game_id: &str,
    user_id: &str,
    score: i64,
    words_found: i64,
    rank_position: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE game_participants SET score = ?, words_found = ?, rank_position = ? WHERE game_id = ? AND user_id = ?",
    )
    .bind(score)
    .bind(words_found)
    .bind(rank_position)
    .bind(game_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Found words ─────────────────────────────────────────────────────────

pub async fn insert_found_word(
    pool: &DbPool,
    id: &Uuid,
    game_id: &str,
    user_id: &str,
    word: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO found_words (id, game_id, user_id, word) VALUES (?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(game_id)
    .bind(user_id)
    .bind(word)
    .execute(pool)
    .await?;
    Ok(())
}

// ── User stats ──────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct UserStatsRow {
    pub user_id: String,
    pub total_games: i64,
    pub games_won: i64,
    pub total_score: i64,
    pub best_single_game_score: i64,
    pub total_words_found: i64,
    pub updated_at: String,
}

pub async fn upsert_user_stats(pool: &DbPool, user_id: &str) -> Result<(), sqlx::Error> {
    // Recalculate from actual data
    let stats: Option<(i64, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT
            COUNT(*) as total_games,
            COALESCE(SUM(CASE WHEN rank_position = 1 THEN 1 ELSE 0 END), 0) as games_won,
            COALESCE(SUM(score), 0) as total_score,
            COALESCE(MAX(score), 0) as best_score,
            COALESCE(SUM(words_found), 0) as total_words
         FROM game_participants WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let (total_games, games_won, total_score, best_score, total_words) =
        stats.unwrap_or((0, 0, 0, 0, 0));

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO user_stats (user_id, total_games, games_won, total_score, best_single_game_score, total_words_found, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET
            total_games = excluded.total_games,
            games_won = excluded.games_won,
            total_score = excluded.total_score,
            best_single_game_score = excluded.best_single_game_score,
            total_words_found = excluded.total_words_found,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind(total_games)
    .bind(games_won)
    .bind(total_score)
    .bind(best_score)
    .bind(total_words)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_stats(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<UserStatsRow>, sqlx::Error> {
    sqlx::query_as::<_, UserStatsRow>(
        "SELECT user_id, total_games, games_won, total_score, best_single_game_score, total_words_found, updated_at FROM user_stats WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

// ── Leaderboard ─────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub username: String,
    pub total_score: i64,
    pub games_won: i64,
    pub total_games: i64,
}

pub async fn get_leaderboard(
    pool: &DbPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    sqlx::query_as::<_, LeaderboardEntry>(
        "SELECT u.username, s.total_score, s.games_won, s.total_games
         FROM user_stats s JOIN users u ON s.user_id = u.id
         ORDER BY s.total_score DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

// ── Game history ────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct GameHistoryRow {
    pub game_id: String,
    pub theme: String,
    pub difficulty: String,
    pub score: i64,
    pub words_found: i64,
    pub rank_position: Option<i64>,
    pub finished_at: Option<String>,
}

pub async fn get_game_history(
    pool: &DbPool,
    user_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<GameHistoryRow>, sqlx::Error> {
    sqlx::query_as::<_, GameHistoryRow>(
        "SELECT g.id as game_id, g.theme, g.difficulty, p.score, p.words_found, p.rank_position, g.finished_at
         FROM game_participants p JOIN games g ON p.game_id = g.id
         WHERE p.user_id = ?
         ORDER BY g.finished_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}
