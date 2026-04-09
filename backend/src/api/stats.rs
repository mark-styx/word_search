use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::db::repository::{self, DbPool};

#[derive(Deserialize)]
struct Pagination {
    page: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct LeaderboardResponse {
    entries: Vec<LeaderboardEntry>,
    total: i64,
}

#[derive(Serialize)]
struct LeaderboardEntry {
    username: String,
    total_score: i64,
    games_won: i64,
    total_games: i64,
}

#[derive(Serialize)]
struct UserStatsResponse {
    total_games: i64,
    games_won: i64,
    total_score: i64,
    best_single_game_score: i64,
    total_words_found: i64,
}

#[derive(Serialize)]
struct GameHistoryEntry {
    game_id: String,
    theme: String,
    difficulty: String,
    score: i64,
    words_found: i64,
    rank: Option<i64>,
    finished_at: Option<String>,
}

#[derive(Serialize)]
struct HistoryResponse {
    entries: Vec<GameHistoryEntry>,
    total: i64,
}

pub fn routes() -> Router {
    Router::new()
        .route("/leaderboard", get(leaderboard))
        .route("/me", get(my_stats))
        .route("/me/history", get(my_history))
}

async fn leaderboard(
    Extension(pool): Extension<DbPool>,
    Query(params): Query<Pagination>,
) -> Result<Json<LeaderboardResponse>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (params.page.unwrap_or(1) - 1) * limit;

    let entries = repository::get_leaderboard(&pool, limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = entries.len() as i64; // simplified; real impl would COUNT

    Ok(Json(LeaderboardResponse {
        entries: entries
            .into_iter()
            .map(|e| LeaderboardEntry {
                username: e.username,
                total_score: e.total_score,
                games_won: e.games_won,
                total_games: e.total_games,
            })
            .collect(),
        total,
    }))
}

async fn my_stats(
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserStatsResponse>, (StatusCode, String)> {
    let stats = repository::get_user_stats(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Stats not found".into()))?;

    Ok(Json(UserStatsResponse {
        total_games: stats.total_games,
        games_won: stats.games_won,
        total_score: stats.total_score,
        best_single_game_score: stats.best_single_game_score,
        total_words_found: stats.total_words_found,
    }))
}

async fn my_history(
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<Pagination>,
) -> Result<Json<HistoryResponse>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (params.page.unwrap_or(1) - 1) * limit;

    let rows = repository::get_game_history(&pool, &auth_user.user_id, limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = rows.len() as i64;

    Ok(Json(HistoryResponse {
        entries: rows
            .into_iter()
            .map(|r| GameHistoryEntry {
                game_id: r.game_id,
                theme: r.theme,
                difficulty: r.difficulty,
                score: r.score,
                words_found: r.words_found,
                rank: r.rank_position,
                finished_at: r.finished_at,
            })
            .collect(),
        total,
    }))
}
