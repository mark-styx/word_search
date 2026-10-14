use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Sqlite};
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
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                let _: String = sqlx::query_scalar("PRAGMA journal_mode = WAL")
                    .fetch_one(&mut *connection)
                    .await?;
                sqlx::query("PRAGMA busy_timeout = 5000")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
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

    let migration_sql = include_str!("../../migrations/006_custom_word_banks.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/007_contested_word_events.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/008_builtin_word_bank_usage.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/009_found_word_claims.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    let migration_sql = include_str!("../../migrations/010_query_indexes.sql");
    sqlx::query(migration_sql).execute(pool).await?;

    ensure_user_highlight_color_column(pool).await?;
    ensure_user_recovery_pin_column(pool).await?;
    ensure_user_is_guest_column(pool).await?;
    ensure_game_state_json_column(pool).await?;
    ensure_custom_word_bank_sharing_column(pool).await?;
    ensure_builtin_word_bank_foreign_keys(pool).await?;

    Ok(())
}

const WORD_USAGE_BATCH_SIZE: usize = 100;

fn aggregate_word_usage(words: &[String]) -> Vec<(String, i64)> {
    let mut counts = std::collections::BTreeMap::new();
    for word in words {
        *counts.entry(word.clone()).or_insert(0_i64) += 1;
    }
    counts.into_iter().collect()
}

async fn ensure_user_highlight_color_column(pool: &DbPool) -> Result<(), sqlx::Error> {
    let columns: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_table_info('users')")
        .fetch_all(pool)
        .await?;

    let has_highlight_color = columns.iter().any(|column| column.0 == "highlight_color");
    if !has_highlight_color {
        sqlx::query("ALTER TABLE users ADD COLUMN highlight_color TEXT NOT NULL DEFAULT '#3b82f6'")
            .execute(pool)
            .await?;
    }

    Ok(())
}

async fn ensure_user_recovery_pin_column(pool: &DbPool) -> Result<(), sqlx::Error> {
    let columns: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_table_info('users')")
        .fetch_all(pool)
        .await?;

    let has_recovery_pin_hash = columns.iter().any(|column| column.0 == "recovery_pin_hash");
    if !has_recovery_pin_hash {
        sqlx::query("ALTER TABLE users ADD COLUMN recovery_pin_hash TEXT")
            .execute(pool)
            .await?;
    }

    Ok(())
}

async fn ensure_user_is_guest_column(pool: &DbPool) -> Result<(), sqlx::Error> {
    let columns: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_table_info('users')")
        .fetch_all(pool)
        .await?;

    let has_is_guest = columns.iter().any(|column| column.0 == "is_guest");
    if !has_is_guest {
        sqlx::query("ALTER TABLE users ADD COLUMN is_guest INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

    Ok(())
}

async fn ensure_game_state_json_column(pool: &DbPool) -> Result<(), sqlx::Error> {
    let columns: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_table_info('games')")
        .fetch_all(pool)
        .await?;

    let has_state_json = columns.iter().any(|column| column.0 == "state_json");
    if !has_state_json {
        sqlx::query("ALTER TABLE games ADD COLUMN state_json TEXT")
            .execute(pool)
            .await?;
    }

    Ok(())
}

async fn builtin_word_bank_table_has_state_fk(
    pool: &DbPool,
    table_name: &str,
) -> Result<bool, sqlx::Error> {
    let query = format!(
        "SELECT \"from\", \"table\" FROM pragma_foreign_key_list('{table_name}')"
    );
    let rows: Vec<(String, String)> = sqlx::query_as(&query).fetch_all(pool).await?;
    Ok(rows
        .iter()
        .any(|(from, table)| from == "theme_key" && table == "builtin_word_bank_state"))
}

async fn rebuild_builtin_word_bank_generated_words_with_fk(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO builtin_word_bank_state (theme_key, can_extend, extension_state, updated_at)
         SELECT DISTINCT g.theme_key, 1, 'idle', datetime('now')
         FROM builtin_word_bank_generated_words g
         LEFT JOIN builtin_word_bank_state s ON s.theme_key = g.theme_key
         WHERE s.theme_key IS NULL",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "ALTER TABLE builtin_word_bank_generated_words
         RENAME TO builtin_word_bank_generated_words_old",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "CREATE TABLE builtin_word_bank_generated_words (
            theme_key TEXT NOT NULL REFERENCES builtin_word_bank_state(theme_key) ON DELETE CASCADE,
            word TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (theme_key, word)
        )",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO builtin_word_bank_generated_words (theme_key, word, created_at)
         SELECT theme_key, word, created_at
         FROM builtin_word_bank_generated_words_old",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query("DROP TABLE builtin_word_bank_generated_words_old")
        .execute(&mut **tx)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_builtin_word_bank_generated_words_theme
         ON builtin_word_bank_generated_words(theme_key, created_at DESC)",
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn rebuild_builtin_word_bank_word_usage_with_fk(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO builtin_word_bank_state (theme_key, can_extend, extension_state, updated_at)
         SELECT DISTINCT u.theme_key, 1, 'idle', datetime('now')
         FROM builtin_word_bank_word_usage u
         LEFT JOIN builtin_word_bank_state s ON s.theme_key = u.theme_key
         WHERE s.theme_key IS NULL",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "ALTER TABLE builtin_word_bank_word_usage
         RENAME TO builtin_word_bank_word_usage_old",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "CREATE TABLE builtin_word_bank_word_usage (
            theme_key TEXT NOT NULL REFERENCES builtin_word_bank_state(theme_key) ON DELETE CASCADE,
            word TEXT NOT NULL,
            times_used INTEGER NOT NULL DEFAULT 0,
            last_used_at TEXT,
            PRIMARY KEY (theme_key, word)
        )",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO builtin_word_bank_word_usage (theme_key, word, times_used, last_used_at)
         SELECT theme_key, word, times_used, last_used_at
         FROM builtin_word_bank_word_usage_old",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query("DROP TABLE builtin_word_bank_word_usage_old")
        .execute(&mut **tx)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_builtin_word_bank_word_usage_theme
         ON builtin_word_bank_word_usage(theme_key, times_used DESC, last_used_at DESC)",
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn ensure_builtin_word_bank_foreign_keys(pool: &DbPool) -> Result<(), sqlx::Error> {
    let generated_words_has_fk =
        builtin_word_bank_table_has_state_fk(pool, "builtin_word_bank_generated_words").await?;
    let usage_has_fk =
        builtin_word_bank_table_has_state_fk(pool, "builtin_word_bank_word_usage").await?;

    if generated_words_has_fk && usage_has_fk {
        return Ok(());
    }

    let mut tx = pool.begin().await?;

    if !generated_words_has_fk {
        rebuild_builtin_word_bank_generated_words_with_fk(&mut tx).await?;
    }

    if !usage_has_fk {
        rebuild_builtin_word_bank_word_usage_with_fk(&mut tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn ensure_custom_word_bank_sharing_column(pool: &DbPool) -> Result<(), sqlx::Error> {
    let columns: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('custom_word_banks')")
            .fetch_all(pool)
            .await?;

    let has_is_shared = columns.iter().any(|column| column.0 == "is_shared");
    if !has_is_shared {
        sqlx::query("ALTER TABLE custom_word_banks ADD COLUMN is_shared INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

    Ok(())
}

// ── User operations ─────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub recovery_pin_hash: Option<String>,
    pub is_guest: i64,
    pub created_at: String,
    pub visual_theme: String,
    pub preferred_word_theme: Option<String>,
    pub highlight_color: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PublicUserRow {
    pub id: String,
    pub username: String,
    pub created_at: String,
}

pub async fn create_user(
    pool: &DbPool,
    id: &Uuid,
    username: &str,
    password_hash: &str,
    recovery_pin_hash: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, recovery_pin_hash) VALUES (?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(username)
    .bind(password_hash)
    .bind(recovery_pin_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_guest_user(
    pool: &DbPool,
    id: &Uuid,
    username: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, recovery_pin_hash, is_guest) VALUES (?, ?, ?, NULL, 1)",
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
        "SELECT id, username, password_hash, recovery_pin_hash, is_guest, created_at, visual_theme, preferred_word_theme, highlight_color FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_id(pool: &DbPool, id: &str) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, username, password_hash, recovery_pin_hash, is_guest, created_at, visual_theme, preferred_word_theme, highlight_color FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn search_usernames(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let pattern = format!("{}%", query.trim());
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT username FROM users WHERE recovery_pin_hash IS NOT NULL AND is_guest = 0 AND username LIKE ? COLLATE NOCASE ORDER BY username ASC LIMIT ?",
    )
    .bind(pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| row.0).collect())
}

pub async fn list_public_users(
    pool: &DbPool,
    query: Option<&str>,
    limit: i64,
) -> Result<Vec<PublicUserRow>, sqlx::Error> {
    let safe_limit = limit.clamp(1, 100);

    if let Some(raw_query) = query {
        let trimmed = raw_query.trim();
        if !trimmed.is_empty() {
            let pattern = format!("%{}%", trimmed);
            return sqlx::query_as::<_, PublicUserRow>(
                "SELECT id, username, created_at
                 FROM users
                 WHERE is_guest = 0 AND username LIKE ? COLLATE NOCASE
                 ORDER BY username COLLATE NOCASE ASC
                 LIMIT ?",
            )
            .bind(pattern)
            .bind(safe_limit)
            .fetch_all(pool)
            .await;
        }
    }

    sqlx::query_as::<_, PublicUserRow>(
        "SELECT id, username, created_at
         FROM users
         WHERE is_guest = 0
         ORDER BY datetime(created_at) DESC, username COLLATE NOCASE ASC
         LIMIT ?",
    )
    .bind(safe_limit)
    .fetch_all(pool)
    .await
}

pub async fn update_user_preferences(
    pool: &DbPool,
    user_id: &str,
    visual_theme: &str,
    preferred_word_theme: Option<&str>,
    highlight_color: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET visual_theme = ?, preferred_word_theme = ?, highlight_color = ? WHERE id = ?",
    )
    .bind(visual_theme)
    .bind(preferred_word_theme)
    .bind(highlight_color)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_user_username(
    pool: &DbPool,
    user_id: &str,
    username: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(username)
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

pub async fn upsert_game_state_snapshot(
    pool: &DbPool,
    id: &str,
    state_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE games SET state_json = ? WHERE id = ?")
        .bind(state_json)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_game_state_snapshot(
    pool: &DbPool,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT state_json FROM games WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|value| value.0))
}

pub async fn list_active_game_state_snapshots(pool: &DbPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT state_json FROM games WHERE status IN ('lobby', 'in_progress') AND state_json IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| row.0).collect())
}

pub async fn update_game_status(pool: &DbPool, id: &str, status: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let finished = if status == "finished" {
        Some(now)
    } else {
        None
    };
    sqlx::query("UPDATE games SET status = ?, finished_at = COALESCE(?, finished_at) WHERE id = ?")
        .bind(status)
        .bind(finished)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_game_started(pool: &DbPool, id: &str) -> Result<(), sqlx::Error> {
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
    sqlx::query("INSERT INTO game_participants (id, game_id, user_id) VALUES (?, ?, ?)")
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
    found_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO found_words (id, game_id, user_id, word, found_at) VALUES (?, ?, ?, ?, ?)",
    )
        .bind(id.to_string())
        .bind(game_id)
        .bind(user_id)
        .bind(word)
        .bind(found_at)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn try_insert_found_word(
    pool: &DbPool,
    id: &Uuid,
    game_id: &str,
    user_id: &str,
    word: &str,
    found_at: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO found_words (id, game_id, user_id, word, found_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(game_id, word) DO NOTHING",
    )
    .bind(id.to_string())
    .bind(game_id)
    .bind(user_id)
    .bind(word)
    .bind(found_at)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FoundWordClaimRow {
    pub user_id: String,
    pub found_at: String,
}

#[derive(Debug, Clone)]
pub enum ClaimFoundWordResult {
    Won {
        previous_claim: Option<FoundWordClaimRow>,
    },
    Lost {
        winning_claim: FoundWordClaimRow,
    },
}

fn claim_sort_key<'a>(user_id: &'a str, found_at: &'a str) -> (&'a str, &'a str) {
    (found_at, user_id)
}

fn challenger_should_replace_claim(
    existing_claim: &FoundWordClaimRow,
    challenger_user_id: &str,
    challenger_found_at: &str,
) -> bool {
    claim_sort_key(challenger_user_id, challenger_found_at)
        < claim_sort_key(&existing_claim.user_id, &existing_claim.found_at)
}

pub async fn claim_found_word(
    pool: &DbPool,
    id: &Uuid,
    game_id: &str,
    user_id: &str,
    word: &str,
    found_at: &str,
) -> Result<ClaimFoundWordResult, sqlx::Error> {
    let previous_claim = get_found_word_claim(pool, game_id, word).await?;

    if previous_claim
        .as_ref()
        .is_some_and(|existing| !challenger_should_replace_claim(existing, user_id, found_at))
    {
        return Ok(ClaimFoundWordResult::Lost {
            winning_claim: previous_claim.expect("checked above"),
        });
    }

    sqlx::query(
        "INSERT INTO found_words (id, game_id, user_id, word, found_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(game_id, word) DO UPDATE SET
            id = excluded.id,
            user_id = excluded.user_id,
            found_at = excluded.found_at
         WHERE found_words.found_at > excluded.found_at
            OR (found_words.found_at = excluded.found_at AND found_words.user_id > excluded.user_id)",
    )
    .bind(id.to_string())
    .bind(game_id)
    .bind(user_id)
    .bind(word)
    .bind(found_at)
    .execute(pool)
    .await?;

    let winning_claim = get_found_word_claim(pool, game_id, word)
        .await?
        .expect("claim row should exist after upsert");

    if winning_claim.user_id == user_id && winning_claim.found_at == found_at {
        Ok(ClaimFoundWordResult::Won { previous_claim })
    } else {
        Ok(ClaimFoundWordResult::Lost { winning_claim })
    }
}

pub async fn get_found_word_claim(
    pool: &DbPool,
    game_id: &str,
    word: &str,
) -> Result<Option<FoundWordClaimRow>, sqlx::Error> {
    sqlx::query_as::<_, FoundWordClaimRow>(
        "SELECT user_id, found_at
         FROM found_words
         WHERE game_id = ? AND word = ?",
    )
    .bind(game_id)
    .bind(word)
    .fetch_optional(pool)
    .await
}

pub async fn delete_found_word_claim(
    pool: &DbPool,
    game_id: &str,
    word: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM found_words
         WHERE game_id = ? AND word = ?",
    )
    .bind(game_id)
    .bind(word)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn restore_found_word_claim(
    pool: &DbPool,
    id: &Uuid,
    game_id: &str,
    user_id: &str,
    word: &str,
    found_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO found_words (id, game_id, user_id, word, found_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(game_id, word) DO UPDATE SET
            id = excluded.id,
            user_id = excluded.user_id,
            found_at = excluded.found_at",
    )
    .bind(id.to_string())
    .bind(game_id)
    .bind(user_id)
    .bind(word)
    .bind(found_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_found_word_claim_if_matches(
    pool: &DbPool,
    game_id: &str,
    word: &str,
    user_id: &str,
    found_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM found_words
         WHERE game_id = ? AND word = ? AND user_id = ? AND found_at = ?",
    )
    .bind(game_id)
    .bind(word)
    .bind(user_id)
    .bind(found_at)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Custom word banks ───────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CustomWordBankRow {
    pub id: String,
    pub user_id: String,
    pub owner_username: String,
    pub prompt: String,
    pub label: Option<String>,
    pub words_json: Option<String>,
    pub total_words: i64,
    pub status: String,
    pub is_shared: i64,
    pub can_extend: i64,
    pub extension_state: String,
    pub error_message: Option<String>,
    pub last_extended_at: Option<String>,
    pub last_extension_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub used_words_count: i64,
    pub total_usage_count: i64,
}

pub async fn create_custom_word_bank(
    pool: &DbPool,
    id: &Uuid,
    user_id: &str,
    prompt: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO custom_word_banks (id, user_id, prompt, status, created_at, updated_at) VALUES (?, ?, ?, 'generating', ?, ?)",
    )
    .bind(id.to_string())
    .bind(user_id)
    .bind(prompt)
    .bind(&now)
    .bind(now.clone())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_custom_word_bank_ready(
    pool: &DbPool,
    id: &str,
    label: &str,
    words: &[String],
    can_extend: bool,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let words_json =
        serde_json::to_string(words).map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

    sqlx::query(
        "UPDATE custom_word_banks
         SET label = ?, words_json = ?, total_words = ?, status = 'ready', can_extend = ?, extension_state = CASE WHEN ? THEN extension_state ELSE 'disabled' END, error_message = NULL, last_extension_error = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(label)
    .bind(words_json)
    .bind(words.len() as i64)
    .bind(can_extend)
    .bind(can_extend)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_custom_word_bank_failed(
    pool: &DbPool,
    id: &str,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE custom_word_banks
         SET status = 'failed', error_message = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(error_message)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_custom_word_bank_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<CustomWordBankRow>, sqlx::Error> {
    sqlx::query_as::<_, CustomWordBankRow>(
        "SELECT
            c.id,
            c.user_id,
            owner_user.username AS owner_username,
            c.prompt,
            c.label,
            c.words_json,
            c.total_words,
            c.status,
            c.is_shared,
            c.can_extend,
            c.extension_state,
            c.error_message,
            c.last_extended_at,
            c.last_extension_error,
            c.created_at,
            c.updated_at,
            COALESCE(usage_stats.used_words_count, 0) AS used_words_count,
            COALESCE(usage_stats.total_usage_count, 0) AS total_usage_count
         FROM custom_word_banks c
         JOIN users owner_user ON owner_user.id = c.user_id
         LEFT JOIN (
            SELECT word_bank_id, COUNT(*) AS used_words_count, COALESCE(SUM(times_used), 0) AS total_usage_count
            FROM custom_word_bank_word_usage
            GROUP BY word_bank_id
         ) usage_stats ON usage_stats.word_bank_id = c.id
         WHERE c.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_visible_custom_word_banks(
    pool: &DbPool,
    user_id: &str,
) -> Result<Vec<CustomWordBankRow>, sqlx::Error> {
    sqlx::query_as::<_, CustomWordBankRow>(
        "SELECT
            c.id,
            c.user_id,
            owner_user.username AS owner_username,
            c.prompt,
            c.label,
            c.words_json,
            c.total_words,
            c.status,
            c.is_shared,
            c.can_extend,
            c.extension_state,
            c.error_message,
            c.last_extended_at,
            c.last_extension_error,
            c.created_at,
            c.updated_at,
            COALESCE(usage_stats.used_words_count, 0) AS used_words_count,
            COALESCE(usage_stats.total_usage_count, 0) AS total_usage_count
         FROM custom_word_banks c
         JOIN users owner_user ON owner_user.id = c.user_id
         LEFT JOIN (
            SELECT word_bank_id, COUNT(*) AS used_words_count, COALESCE(SUM(times_used), 0) AS total_usage_count
            FROM custom_word_bank_word_usage
            GROUP BY word_bank_id
         ) usage_stats ON usage_stats.word_bank_id = c.id
         WHERE c.user_id = ? OR (c.status = 'ready' AND c.is_shared = 1)
         ORDER BY
            CASE WHEN c.user_id = ? THEN 0 ELSE 1 END,
            CASE c.status WHEN 'ready' THEN 0 WHEN 'generating' THEN 1 ELSE 2 END,
            c.created_at DESC",
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn update_custom_word_bank_sharing(
    pool: &DbPool,
    id: &str,
    user_id: &str,
    is_shared: bool,
) -> Result<bool, sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE custom_word_banks
         SET is_shared = ?, updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(is_shared)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_custom_word_bank(
    pool: &DbPool,
    id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "DELETE FROM custom_word_bank_word_usage
         WHERE word_bank_id = ? AND EXISTS (
            SELECT 1 FROM custom_word_banks WHERE id = ? AND user_id = ?
         )",
    )
    .bind(id)
    .bind(id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query(
        "DELETE FROM custom_word_banks
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

pub async fn record_custom_word_bank_usage(
    pool: &DbPool,
    word_bank_id: &str,
    words: &[String],
) -> Result<(), sqlx::Error> {
    if words.is_empty() {
        return Ok(());
    }

    let usage_counts = aggregate_word_usage(words);
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    for chunk in usage_counts.chunks(WORD_USAGE_BATCH_SIZE) {
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT INTO custom_word_bank_word_usage (word_bank_id, word, times_used, last_used_at) ",
        );
        query_builder.push_values(chunk, |mut builder, (word, times_used)| {
            builder
                .push_bind(word_bank_id)
                .push_bind(word)
                .push_bind(*times_used)
                .push_bind(&now);
        });
        query_builder.push(
            " ON CONFLICT(word_bank_id, word) DO UPDATE SET
                times_used = custom_word_bank_word_usage.times_used + excluded.times_used,
                last_used_at = excluded.last_used_at",
        );
        query_builder.build().execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CustomWordUsageRow {
    pub word: String,
    pub times_used: i64,
    pub last_used_at: Option<String>,
}

pub async fn list_custom_word_bank_word_usage(
    pool: &DbPool,
    word_bank_id: &str,
) -> Result<Vec<CustomWordUsageRow>, sqlx::Error> {
    sqlx::query_as::<_, CustomWordUsageRow>(
        "SELECT word, times_used, last_used_at
         FROM custom_word_bank_word_usage
         WHERE word_bank_id = ?",
    )
    .bind(word_bank_id)
    .fetch_all(pool)
    .await
}

pub async fn update_custom_word_bank_extension_state(
    pool: &DbPool,
    id: &str,
    extension_state: &str,
    can_extend: bool,
    last_extension_error: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE custom_word_banks
         SET extension_state = ?, can_extend = ?, last_extension_error = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(extension_state)
    .bind(can_extend)
    .bind(last_extension_error)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn append_custom_word_bank_words(
    pool: &DbPool,
    id: &str,
    words: &[String],
    can_extend: bool,
) -> Result<(), sqlx::Error> {
    let row = get_custom_word_bank_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    let existing = row.words_json.unwrap_or_else(|| "[]".into());
    let mut current_words: Vec<String> = serde_json::from_str(&existing)
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

    let mut seen: std::collections::HashSet<String> = current_words.iter().cloned().collect();
    for word in words {
        if seen.insert(word.clone()) {
            current_words.push(word.clone());
        }
    }

    let words_json = serde_json::to_string(&current_words)
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE custom_word_banks
         SET words_json = ?, total_words = ?, can_extend = ?, extension_state = CASE WHEN ? THEN 'idle' ELSE 'disabled' END, last_extended_at = ?, last_extension_error = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(words_json)
    .bind(current_words.len() as i64)
    .bind(can_extend)
    .bind(can_extend)
    .bind(&now)
    .bind(now.clone())
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

// ── Built-in word banks ─────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BuiltInWordBankRow {
    pub theme_key: String,
    pub can_extend: i64,
    pub extension_state: String,
    pub last_extended_at: Option<String>,
    pub last_extension_error: Option<String>,
    pub generated_words_count: i64,
    pub used_words_count: i64,
    pub total_usage_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BuiltInWordUsageRow {
    pub word: String,
    pub times_used: i64,
    pub last_used_at: Option<String>,
}

pub async fn list_builtin_word_banks(
    pool: &DbPool,
) -> Result<Vec<BuiltInWordBankRow>, sqlx::Error> {
    sqlx::query_as::<_, BuiltInWordBankRow>(
        "SELECT
            s.theme_key,
            s.can_extend,
            s.extension_state,
            s.last_extended_at,
            s.last_extension_error,
            COALESCE(g.generated_words_count, 0) AS generated_words_count,
            COALESCE(usage_stats.used_words_count, 0) AS used_words_count,
            COALESCE(usage_stats.total_usage_count, 0) AS total_usage_count
         FROM builtin_word_bank_state s
         LEFT JOIN (
            SELECT theme_key, COUNT(*) AS generated_words_count
            FROM builtin_word_bank_generated_words
            GROUP BY theme_key
         ) g ON g.theme_key = s.theme_key
         LEFT JOIN (
            SELECT theme_key, COUNT(*) AS used_words_count, COALESCE(SUM(times_used), 0) AS total_usage_count
            FROM builtin_word_bank_word_usage
            GROUP BY theme_key
         ) usage_stats ON usage_stats.theme_key = s.theme_key",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_builtin_word_bank_by_theme(
    pool: &DbPool,
    theme_key: &str,
) -> Result<Option<BuiltInWordBankRow>, sqlx::Error> {
    sqlx::query_as::<_, BuiltInWordBankRow>(
        "SELECT
            s.theme_key,
            s.can_extend,
            s.extension_state,
            s.last_extended_at,
            s.last_extension_error,
            COALESCE(g.generated_words_count, 0) AS generated_words_count,
            COALESCE(u.used_words_count, 0) AS used_words_count,
            COALESCE(u.total_usage_count, 0) AS total_usage_count
         FROM builtin_word_bank_state s
         LEFT JOIN (
            SELECT theme_key, COUNT(*) AS generated_words_count
            FROM builtin_word_bank_generated_words
            GROUP BY theme_key
         ) g ON g.theme_key = s.theme_key
         LEFT JOIN (
            SELECT theme_key, COUNT(*) AS used_words_count, COALESCE(SUM(times_used), 0) AS total_usage_count
            FROM builtin_word_bank_word_usage
            GROUP BY theme_key
         ) u ON u.theme_key = s.theme_key
         WHERE s.theme_key = ?",
    )
    .bind(theme_key)
    .fetch_optional(pool)
    .await
}

pub async fn ensure_builtin_word_bank_state(
    pool: &DbPool,
    theme_key: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO builtin_word_bank_state (theme_key, can_extend, extension_state, updated_at)
         VALUES (?, 1, 'idle', ?)
         ON CONFLICT(theme_key) DO NOTHING",
    )
    .bind(theme_key)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_builtin_generated_words(
    pool: &DbPool,
    theme_key: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT word
         FROM builtin_word_bank_generated_words
         WHERE theme_key = ?
         ORDER BY datetime(created_at) ASC, word ASC",
    )
    .bind(theme_key)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| row.0).collect())
}

pub async fn record_builtin_word_bank_usage(
    pool: &DbPool,
    theme_key: &str,
    words: &[String],
) -> Result<(), sqlx::Error> {
    if words.is_empty() {
        return Ok(());
    }

    let usage_counts = aggregate_word_usage(words);
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO builtin_word_bank_state (theme_key, can_extend, extension_state, updated_at)
         VALUES (?, 1, 'idle', ?)
         ON CONFLICT(theme_key) DO NOTHING",
    )
    .bind(theme_key)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    for chunk in usage_counts.chunks(WORD_USAGE_BATCH_SIZE) {
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT INTO builtin_word_bank_word_usage (theme_key, word, times_used, last_used_at) ",
        );
        query_builder.push_values(chunk, |mut builder, (word, times_used)| {
            builder
                .push_bind(theme_key)
                .push_bind(word)
                .push_bind(*times_used)
                .push_bind(&now);
        });
        query_builder.push(
            " ON CONFLICT(theme_key, word) DO UPDATE SET
                times_used = builtin_word_bank_word_usage.times_used + excluded.times_used,
                last_used_at = excluded.last_used_at",
        );
        query_builder.build().execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_builtin_word_bank_word_usage(
    pool: &DbPool,
    theme_key: &str,
) -> Result<Vec<BuiltInWordUsageRow>, sqlx::Error> {
    sqlx::query_as::<_, BuiltInWordUsageRow>(
        "SELECT word, times_used, last_used_at
         FROM builtin_word_bank_word_usage
         WHERE theme_key = ?",
    )
    .bind(theme_key)
    .fetch_all(pool)
    .await
}

pub async fn update_builtin_word_bank_extension_state(
    pool: &DbPool,
    theme_key: &str,
    extension_state: &str,
    can_extend: bool,
    last_extension_error: Option<&str>,
) -> Result<(), sqlx::Error> {
    ensure_builtin_word_bank_state(pool, theme_key).await?;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE builtin_word_bank_state
         SET extension_state = ?, can_extend = ?, last_extension_error = ?, updated_at = ?
         WHERE theme_key = ?",
    )
    .bind(extension_state)
    .bind(can_extend)
    .bind(last_extension_error)
    .bind(now)
    .bind(theme_key)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn append_builtin_word_bank_words(
    pool: &DbPool,
    theme_key: &str,
    words: &[String],
    can_extend: bool,
) -> Result<(), sqlx::Error> {
    ensure_builtin_word_bank_state(pool, theme_key).await?;
    let now = chrono::Utc::now().to_rfc3339();

    for word in words {
        sqlx::query(
            "INSERT INTO builtin_word_bank_generated_words (theme_key, word, created_at)
             VALUES (?, ?, ?)
             ON CONFLICT(theme_key, word) DO NOTHING",
        )
        .bind(theme_key)
        .bind(word)
        .bind(&now)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        "UPDATE builtin_word_bank_state
         SET can_extend = ?, extension_state = CASE WHEN ? THEN 'idle' ELSE 'disabled' END,
             last_extended_at = ?, last_extension_error = NULL, updated_at = ?
         WHERE theme_key = ?",
    )
    .bind(can_extend)
    .bind(can_extend)
    .bind(&now)
    .bind(now.clone())
    .bind(theme_key)
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
    pub contested_words_won: i64,
    pub contested_words_lost: i64,
    pub closest_contest_ms: Option<i64>,
    pub fastest_word_find_ms: Option<i64>,
    pub average_word_find_ms: Option<i64>,
    pub updated_at: String,
}

pub async fn record_contested_word_event(
    pool: &DbPool,
    id: &Uuid,
    game_id: &str,
    word: &str,
    winning_user_id: &str,
    losing_user_id: &str,
    winning_found_at: &str,
    losing_attempted_at: &str,
    delta_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO contested_word_events (
            id, game_id, word, winning_user_id, losing_user_id, winning_found_at, losing_attempted_at, delta_ms, attempts
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)
         ON CONFLICT(game_id, word, winning_user_id, losing_user_id) DO UPDATE SET
            delta_ms = MIN(contested_word_events.delta_ms, excluded.delta_ms),
            losing_attempted_at = CASE
                WHEN excluded.delta_ms <= contested_word_events.delta_ms THEN excluded.losing_attempted_at
                ELSE contested_word_events.losing_attempted_at
            END,
            attempts = contested_word_events.attempts + 1",
    )
    .bind(id.to_string())
    .bind(game_id)
    .bind(word)
    .bind(winning_user_id)
    .bind(losing_user_id)
    .bind(winning_found_at)
    .bind(losing_attempted_at)
    .bind(delta_ms)
    .execute(pool)
    .await?;

    Ok(())
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
        "SELECT
            s.user_id,
            s.total_games,
            s.games_won,
            s.total_score,
            s.best_single_game_score,
            s.total_words_found,
            COALESCE((
                SELECT COUNT(*)
                FROM contested_word_events c
                WHERE c.winning_user_id = s.user_id
            ), 0) AS contested_words_won,
            COALESCE((
                SELECT COUNT(*)
                FROM contested_word_events c
                WHERE c.losing_user_id = s.user_id
            ), 0) AS contested_words_lost,
            (
                SELECT MIN(delta_ms)
                FROM contested_word_events c
                WHERE c.winning_user_id = s.user_id OR c.losing_user_id = s.user_id
            ) AS closest_contest_ms,
            (
                SELECT MIN(CAST(ROUND((julianday(f.found_at) - julianday(g.started_at)) * 86400000) AS INTEGER))
                FROM found_words f
                JOIN games g ON g.id = f.game_id
                WHERE f.user_id = s.user_id AND g.started_at IS NOT NULL
            ) AS fastest_word_find_ms,
            (
                SELECT CAST(ROUND(AVG((julianday(f.found_at) - julianday(g.started_at)) * 86400000)) AS INTEGER)
                FROM found_words f
                JOIN games g ON g.id = f.game_id
                WHERE f.user_id = s.user_id AND g.started_at IS NOT NULL
            ) AS average_word_find_ms,
            s.updated_at
         FROM user_stats s
         WHERE s.user_id = ?",
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
         WHERE u.is_guest = 0
         ORDER BY s.total_score DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn get_leaderboard_count(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)
         FROM user_stats s JOIN users u ON s.user_id = u.id
         WHERE u.is_guest = 0",
    )
    .fetch_one(pool)
    .await?;

    Ok(row.0)
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

pub async fn get_game_history_count(pool: &DbPool, user_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*)
         FROM game_participants p JOIN games g ON p.game_id = g.id
         WHERE p.user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}
