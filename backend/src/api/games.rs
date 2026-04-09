use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::repository::{self, DbPool};
use crate::game::grid::generate_grid;
use crate::models::{Difficulty, GameState, GameStatus, PlayerState, WordTheme};
use crate::puzzles::catalog;

use crate::api::GameRooms;
#[derive(Deserialize)]
struct CreateGameRequest {
    theme: WordTheme,
    difficulty: Difficulty,
    time_limit_secs: Option<u64>,
    max_players: Option<usize>,
}

#[derive(Serialize)]
struct GameResponse {
    id: String,
}

#[derive(Serialize)]
struct GridResponse {
    cells: Vec<Vec<String>>,
    width: usize,
    height: usize,
    placed_words: Vec<PlacedWordResponse>,
}

#[derive(Serialize)]
struct PlacedWordResponse {
    word: String,
    start: (usize, usize),
    end: (usize, usize),
    direction: String,
}

#[derive(Serialize)]
struct GameDetailResponse {
    id: String,
    grid: Option<GridResponse>,
    theme: String,
    difficulty: String,
    status: String,
    players: Vec<PlayerInfo>,
    found_words: HashMap<String, Vec<String>>,
    max_players: usize,
    time_limit_secs: Option<u64>,
    started_at: Option<String>,
}

#[derive(Serialize)]
struct PlayerInfo {
    id: String,
    username: String,
    score: u32,
    is_host: bool,
}

#[derive(Serialize)]
struct LobbyEntry {
    id: String,
    theme: String,
    difficulty: String,
    host: String,
    player_count: usize,
    max_players: usize,
}

pub fn public_routes() -> Router {
    Router::new()
        .route("/:id", get(get_game))
        .route("/", get(list_games))
}

pub fn protected_routes() -> Router {
    Router::new()
        .route("/solo", get(solo_game))
        .route("/:id/join", post(join_game))
        .route("/:id/start", post(start_game))
        .route("/", post(create_game))
}

async fn create_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<CreateGameRequest>,
) -> Result<(StatusCode, Json<GameResponse>), (StatusCode, String)> {
    let game_id = Uuid::new_v4();
    let max_players = body.max_players.unwrap_or(8).max(2).min(8);

    let mut players = HashMap::new();
    players.insert(
        Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4()),
        PlayerState {
            id: Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4()),
            username: auth_user.username.clone(),
            score: 0,
            is_host: true,
        },
    );

    let game_state = GameState {
        id: game_id,
        grid: None,
        theme: body.theme,
        difficulty: body.difficulty,
        players,
        found_words: HashMap::new(),
        status: GameStatus::Lobby,
        started_at: None,
        time_limit: body.time_limit_secs.map(std::time::Duration::from_secs),
        max_players,
    };

    repository::insert_game(
        &pool,
        &game_id,
        &format!("{:?}", game_state.theme),
        &format!("{:?}", game_state.difficulty),
        "lobby",
        body.time_limit_secs.map(|s| s as i64),
        false,
        max_players as i64,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    repository::insert_participant(
        &pool,
        &Uuid::new_v4(),
        &game_id.to_string(),
        &auth_user.user_id,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    rooms.write().await.insert(game_id, game_state);

    Ok((StatusCode::CREATED, Json(GameResponse { id: game_id.to_string() })))
}

async fn list_games(
    Extension(rooms): Extension<GameRooms>,
) -> Json<Vec<LobbyEntry>> {
    let rooms = rooms.read().await;
    let lobbies: Vec<LobbyEntry> = rooms
        .values()
        .filter(|g| g.status == GameStatus::Lobby)
        .map(|g| {
            let host = g
                .players
                .values()
                .find(|p| p.is_host)
                .map(|p| p.username.clone())
                .unwrap_or_default();
            LobbyEntry {
                id: g.id.to_string(),
                theme: format!("{:?}", g.theme),
                difficulty: format!("{:?}", g.difficulty),
                host,
                player_count: g.players.len(),
                max_players: g.max_players,
            }
        })
        .collect();
    Json(lobbies)
}

async fn get_game(
    Extension(rooms): Extension<GameRooms>,
    Path(id): Path<String>,
) -> Result<Json<GameDetailResponse>, (StatusCode, String)> {
    let game_id = Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid game ID".into()))?;
    let rooms = rooms.read().await;
    let game = rooms
        .get(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game not found".into()))?;

    Ok(Json(GameDetailResponse {
        id: game.id.to_string(),
        grid: game.grid.as_ref().map(|g| GridResponse {
            cells: g.cells.iter().map(|row| row.iter().map(|c| c.to_string()).collect()).collect(),
            width: g.width,
            height: g.height,
            placed_words: g.placed_words.iter().map(|pw| PlacedWordResponse {
                word: pw.word.clone(),
                start: pw.start,
                end: pw.end,
                direction: serde_json::to_value(&pw.direction).unwrap().as_str().unwrap().to_string(),
            }).collect(),
        }),
        theme: serde_json::to_value(&game.theme).unwrap().as_str().unwrap().to_string(),
        difficulty: serde_json::to_value(&game.difficulty).unwrap().as_str().unwrap().to_string(),
        status: serde_json::to_value(&game.status).unwrap().as_str().unwrap().to_string(),
        players: game
            .players
            .values()
            .map(|p| PlayerInfo {
                id: p.id.to_string(),
                username: p.username.clone(),
                score: p.score,
                is_host: p.is_host,
            })
            .collect(),
        found_words: game
            .found_words
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        max_players: game.max_players,
        time_limit_secs: game.time_limit.map(|d| d.as_secs()),
        started_at: game.started_at.map(|dt| dt.to_rfc3339()),
    }))
}

async fn join_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<GameResponse>, (StatusCode, String)> {
    let game_id = Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid game ID".into()))?;
    let user_uuid = Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4());

    let mut rooms = rooms.write().await;
    let game = rooms
        .get_mut(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game not found".into()))?;

    if game.status != GameStatus::Lobby {
        return Err((StatusCode::CONFLICT, "Game is not in lobby state".into()));
    }
    if game.players.len() >= game.max_players {
        return Err((StatusCode::CONFLICT, "Game is full".into()));
    }
    if game.players.contains_key(&user_uuid) {
        // Already in game - return success (idempotent)
        return Ok(Json(GameResponse { id: game_id.to_string() }));
    }

    game.players.insert(
        user_uuid,
        PlayerState {
            id: user_uuid,
            username: auth_user.username.clone(),
            score: 0,
            is_host: false,
        },
    );

    repository::insert_participant(
        &pool,
        &Uuid::new_v4(),
        &game_id.to_string(),
        &auth_user.user_id,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(GameResponse { id: game_id.to_string() }))
}

async fn start_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<GameResponse>, (StatusCode, String)> {
    let game_id = Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid game ID".into()))?;
    let user_uuid = Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4());

    let mut rooms = rooms.write().await;
    let game = rooms
        .get_mut(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game not found".into()))?;

    if game.status != GameStatus::Lobby {
        return Err((StatusCode::CONFLICT, "Game is not in lobby state".into()));
    }

    let player = game
        .players
        .get(&user_uuid)
        .ok_or((StatusCode::FORBIDDEN, "You are not in this game".into()))?;
    if !player.is_host {
        return Err((StatusCode::FORBIDDEN, "Only the host can start the game".into()));
    }

    // Generate the puzzle grid
    let words = catalog::get_words(
        game.theme,
        game.difficulty.word_count(),
        Some(game.difficulty.grid_size()),
    );
    let grid = generate_grid(&words, game.difficulty)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    game.grid = Some(grid);
    game.status = GameStatus::InProgress;
    game.started_at = Some(chrono::Utc::now());

    repository::update_game_started(&pool, &game_id.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(GameResponse { id: game_id.to_string() }))
}

async fn solo_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<GameResponse>, (StatusCode, String)> {
    // Solo game: use medium difficulty, random theme
    let game_id = Uuid::new_v4();
    let user_uuid = Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4());
    let theme = WordTheme::Science; // default for solo; could be configurable
    let difficulty = Difficulty::Medium;

    let words = catalog::get_words(theme, difficulty.word_count(), Some(difficulty.grid_size()));
    let grid = generate_grid(&words, difficulty)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut players = HashMap::new();
    players.insert(
        user_uuid,
        PlayerState {
            id: user_uuid,
            username: auth_user.username.clone(),
            score: 0,
            is_host: true,
        },
    );

    let game_state = GameState {
        id: game_id,
        grid: Some(grid),
        theme,
        difficulty,
        players,
        found_words: HashMap::new(),
        status: GameStatus::InProgress,
        started_at: Some(chrono::Utc::now()),
        time_limit: None,
        max_players: 1,
    };

    repository::insert_game(
        &pool,
        &game_id,
        &format!("{:?}", theme),
        &format!("{:?}", difficulty),
        "in_progress",
        None,
        true,
        1,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    repository::insert_participant(
        &pool,
        &Uuid::new_v4(),
        &game_id.to_string(),
        &auth_user.user_id,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    rooms.write().await.insert(game_id, game_state);

    Ok(Json(GameResponse { id: game_id.to_string() }))
}
