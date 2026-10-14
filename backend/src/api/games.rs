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
use crate::models::{
    Difficulty, GameFinishReason, GameState, GameStatus, PlayerState, ServerMessage,
    WordBankSource, WordTheme,
};
use crate::puzzles::catalog;
use crate::word_banks;
use crate::ws::room;

use crate::api::{get_room_handle, room_handle, GameChannels, GameRooms};
#[derive(Deserialize)]
struct CreateGameRequest {
    theme: Option<String>,
    word_bank_id: Option<String>,
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
pub(crate) struct GameDetailResponse {
    id: String,
    grid: Option<GridResponse>,
    theme: String,
    difficulty: String,
    status: String,
    finished_reason: Option<String>,
    players: Vec<PlayerInfo>,
    found_words: HashMap<String, Vec<String>>,
    max_players: usize,
    time_limit_secs: Option<u64>,
    started_at: Option<String>,
    revision: u64,
    updated_at: String,
}

#[derive(Serialize)]
struct PlayerInfo {
    id: String,
    username: String,
    score: u32,
    is_host: bool,
    ready: bool,
    highlight_color: String,
    connected: bool,
}

struct GeneratedWords {
    words: Vec<String>,
    theme_label_override: Option<String>,
}

struct GeneratedGrid {
    grid: crate::models::Grid,
    theme_label_override: Option<String>,
}

#[derive(Serialize)]
struct LobbyEntry {
    id: String,
    theme: String,
    difficulty: String,
    host: String,
    player_count: usize,
    max_players: usize,
    revision: u64,
    updated_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FinishedParticipantUpdate {
    pub player_id: Uuid,
    pub score: i64,
    pub words_found: i64,
    pub rank_position: Option<i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct FinishedGameUpdate {
    pub rankings: Vec<PlayerState>,
    pub participants: Vec<FinishedParticipantUpdate>,
    pub snapshot: GameState,
    pub reason: GameFinishReason,
}

fn grid_response(grid: &crate::models::Grid) -> GridResponse {
    GridResponse {
        cells: grid
            .cells
            .iter()
            .map(|row| row.iter().map(|c| c.to_string()).collect())
            .collect(),
        width: grid.width,
        height: grid.height,
        placed_words: grid
            .placed_words
            .iter()
            .map(|pw| PlacedWordResponse {
                word: pw.word.clone(),
                start: pw.start,
                end: pw.end,
                direction: serde_json::to_value(&pw.direction)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
            })
            .collect(),
    }
}

pub(crate) fn game_detail_response_from_state(game: &GameState) -> GameDetailResponse {
    GameDetailResponse {
        id: game.id.to_string(),
        grid: game.grid.as_ref().map(grid_response),
        theme: serde_json::to_value(&game.theme)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        difficulty: serde_json::to_value(&game.difficulty)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        status: serde_json::to_value(&game.status)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        finished_reason: game.finished_reason.map(|reason| {
            serde_json::to_value(reason)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        }),
        players: game
            .players
            .values()
            .map(|p| PlayerInfo {
                id: p.id.to_string(),
                username: p.username.clone(),
                score: p.score,
                is_host: p.is_host,
                ready: p.ready,
                highlight_color: p.highlight_color.clone(),
                connected: p.connected,
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
        revision: game.revision,
        updated_at: game.updated_at.to_rfc3339(),
    }
}

fn lobby_entry_from_state(game: &GameState) -> Option<LobbyEntry> {
    if game.status != GameStatus::Lobby {
        return None;
    }

    let host = game
        .players
        .values()
        .find(|p| p.is_host)
        .or_else(|| game.players.values().next())
        .map(|p| p.username.clone())
        .unwrap_or_else(|| "Unknown host".to_string());

    Some(LobbyEntry {
        id: game.id.to_string(),
        theme: serde_json::to_value(&game.theme)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        difficulty: serde_json::to_value(&game.difficulty)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        host,
        player_count: game.players.len(),
        max_players: game.max_players,
        revision: game.revision,
        updated_at: game.updated_at.to_rfc3339(),
    })
}

pub(crate) fn game_state_sync_message(game: &GameState) -> crate::models::ServerMessage {
    room::game_state_sync_message(game)
}

pub(crate) async fn persist_game_state_snapshot(pool: &DbPool, game: &GameState) {
    room::persist_game_state_snapshot(pool, game).await;
}

pub(crate) fn build_finished_game_update(
    game: &mut GameState,
    reason: GameFinishReason,
) -> Option<FinishedGameUpdate> {
    if game.status == GameStatus::Finished {
        return None;
    }

    game.status = GameStatus::Finished;
    game.finished_reason = Some(reason);
    game.touch();

    let mut rankings: Vec<PlayerState> = game.players.values().cloned().collect();
    rankings.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.username.cmp(&b.username))
            .then_with(|| a.id.cmp(&b.id))
    });

    let participants = rankings
        .iter()
        .enumerate()
        .map(|(index, player)| FinishedParticipantUpdate {
            player_id: player.id,
            score: player.score as i64,
            words_found: game
                .found_words
                .get(&player.id)
                .map(|words| words.len())
                .unwrap_or(0) as i64,
            rank_position: if rankings.len() > 1 {
                Some(index as i64 + 1)
            } else {
                None
            },
        })
        .collect();

    Some(FinishedGameUpdate {
        rankings,
        participants,
        snapshot: game.clone(),
        reason,
    })
}

pub(crate) async fn finalize_game(
    pool: &DbPool,
    rooms: &GameRooms,
    channels: &GameChannels,
    game_id: Uuid,
    reason: GameFinishReason,
) -> bool {
    let Some(room) = get_room_handle(rooms, game_id).await else {
        return false;
    };

    let Some(finished) = ({
        let mut game = room.write().await;
        build_finished_game_update(&mut game, reason)
    }) else {
        return false;
    };

    let game_id_string = game_id.to_string();
    let _ = repository::update_game_status(pool, &game_id_string, "finished").await;
    for participant in &finished.participants {
        let _ = repository::update_participant_stats(
            pool,
            &game_id_string,
            &participant.player_id.to_string(),
            participant.score,
            participant.words_found,
            participant.rank_position,
        )
        .await;
        let _ = repository::upsert_user_stats(pool, &participant.player_id.to_string()).await;
    }

    persist_game_state_snapshot(pool, &finished.snapshot).await;

    let tx = {
        let channels = channels.read().await;
        channels.get(&game_id).cloned()
    };
    if let Some(tx) = tx {
        let _ = tx.send(ServerMessage::GameFinished {
            rankings: finished.rankings,
            reason: finished.reason,
        });
        let _ = tx.send(game_state_sync_message(&finished.snapshot));
    }

    true
}

pub(crate) async fn schedule_game_timeout_watch(
    pool: DbPool,
    rooms: GameRooms,
    channels: GameChannels,
    game_id: Uuid,
) {
    let Some(room) = get_room_handle(&rooms, game_id).await else {
        return;
    };

    let Some(deadline) = ({
        let game = room.read().await;
        if game.status != GameStatus::InProgress {
            None
        } else if let (Some(started_at), Some(time_limit)) = (game.started_at, game.time_limit) {
            chrono::Duration::from_std(time_limit)
                .ok()
                .map(|duration| started_at + duration)
        } else {
            None
        }
    }) else {
        return;
    };

    let now = chrono::Utc::now();
    if deadline <= now {
        let _ = finalize_game(&pool, &rooms, &channels, game_id, GameFinishReason::Timeout).await;
        return;
    }

    let Ok(remaining) = (deadline - now).to_std() else {
        return;
    };

    tokio::spawn(async move {
        tokio::time::sleep(remaining).await;
        let _ = finalize_game(&pool, &rooms, &channels, game_id, GameFinishReason::Timeout).await;
    });
}

pub(crate) async fn ensure_room_loaded(
    pool: &DbPool,
    rooms: &GameRooms,
    game_id: Uuid,
) -> Result<bool, String> {
    room::ensure_room_loaded(pool, rooms, game_id).await
}

pub(crate) async fn warm_active_rooms_from_snapshots(
    pool: &DbPool,
    rooms: &GameRooms,
) -> Result<(), String> {
    room::warm_active_rooms_from_snapshots(pool, rooms).await
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
    let word_bank = resolve_requested_word_bank(
        &pool,
        &auth_user.user_id,
        body.theme.as_deref(),
        body.word_bank_id.as_deref(),
    )
    .await?;
    let user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    let mut players = HashMap::new();
    players.insert(
        Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4()),
        PlayerState {
            id: Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4()),
            username: auth_user.username.clone(),
            score: 0,
            is_host: true,
            ready: false,
            highlight_color: user.highlight_color,
            connected: true,
        },
    );

    let game_state = GameState {
        id: game_id,
        grid: None,
        theme: word_bank.label.clone(),
        word_bank_source: word_bank.source,
        difficulty: body.difficulty,
        players,
        found_words: HashMap::new(),
        status: GameStatus::Lobby,
        finished_reason: None,
        started_at: None,
        time_limit: body.time_limit_secs.map(std::time::Duration::from_secs),
        max_players,
        revision: 1,
        updated_at: chrono::Utc::now(),
        word_claims: HashMap::new(),
    };

    repository::insert_game(
        &pool,
        &game_id,
        &word_bank.label,
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

    rooms
        .write()
        .await
        .insert(game_id, room_handle(game_state.clone()));
    persist_game_state_snapshot(&pool, &game_state).await;

    Ok((
        StatusCode::CREATED,
        Json(GameResponse {
            id: game_id.to_string(),
        }),
    ))
}

async fn list_games(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
) -> Json<Vec<LobbyEntry>> {
    if let Err(error) = warm_active_rooms_from_snapshots(&pool, &rooms).await {
        tracing::warn!(error = %error, "failed to warm active rooms from snapshots");
    }

    let room_handles: Vec<_> = rooms.read().await.values().cloned().collect();
    let mut lobbies = Vec::new();
    for room in room_handles {
        let game = room.read().await;
        if let Some(entry) = lobby_entry_from_state(&game) {
            lobbies.push(entry);
        }
    }
    Json(lobbies)
}

async fn get_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Path(id): Path<String>,
) -> Result<Json<GameDetailResponse>, (StatusCode, String)> {
    let game_id =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid game ID".into()))?;
    ensure_room_loaded(&pool, &rooms, game_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let room = get_room_handle(&rooms, game_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Game not found".into()))?;
    let game = room.read().await;

    Ok(Json(game_detail_response_from_state(&game)))
}

async fn join_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(channels): Extension<GameChannels>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<GameResponse>, (StatusCode, String)> {
    let game_id =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid game ID".into()))?;
    ensure_room_loaded(&pool, &rooms, game_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let user_uuid = Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4());
    let game_id_string = game_id.to_string();
    let user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    let player_state = PlayerState {
        id: user_uuid,
        username: auth_user.username.clone(),
        score: 0,
        is_host: false,
        ready: false,
        highlight_color: user.highlight_color,
        connected: true,
    };

    let room = get_room_handle(&rooms, game_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Game not found".into()))?;

    {
        let mut game = room.write().await;

        if game.status != GameStatus::Lobby {
            return Err((StatusCode::CONFLICT, "Game is not in lobby state".into()));
        }
        if game.players.len() >= game.max_players {
            return Err((StatusCode::CONFLICT, "Game is full".into()));
        }
        if game.players.contains_key(&user_uuid) {
            return Ok(Json(GameResponse { id: game_id_string }));
        }

        game.players.insert(user_uuid, player_state.clone());
        game.touch();
    }

    if let Err(error) = repository::insert_participant(
        &pool,
        &Uuid::new_v4(),
        &game_id_string,
        &auth_user.user_id,
    )
    .await
    {
        if let Some(room) = get_room_handle(&rooms, game_id).await {
            let mut game = room.write().await;
            if game.status == GameStatus::Lobby && game.players.remove(&user_uuid).is_some() {
                game.touch();
            }
        }
        return Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string()));
    }

    let channels = channels.read().await;
    if let Some(tx) = channels.get(&game_id) {
        let _ = tx.send(ServerMessage::PlayerJoined {
            player: player_state,
        });
        if let Some(room) = get_room_handle(&rooms, game_id).await {
            let game = room.read().await;
            let _ = tx.send(game_state_sync_message(&game));
        }
    }

    if let Some(room) = get_room_handle(&rooms, game_id).await {
        let game = room.read().await;
        persist_game_state_snapshot(&pool, &game).await;
    }

    Ok(Json(GameResponse {
        id: game_id.to_string(),
    }))
}

async fn start_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(channels): Extension<GameChannels>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<GameResponse>, (StatusCode, String)> {
    let game_id =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid game ID".into()))?;
    ensure_room_loaded(&pool, &rooms, game_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let user_uuid = Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4());
    let room = get_room_handle(&rooms, game_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Game not found".into()))?;
    let (word_bank_source, difficulty, time_limit) = {
        let game = room.read().await;

        if game.status != GameStatus::Lobby {
            return Err((StatusCode::CONFLICT, "Game is not in lobby state".into()));
        }

        let player = game
            .players
            .get(&user_uuid)
            .ok_or((StatusCode::FORBIDDEN, "You are not in this game".into()))?;
        if !player.is_host {
            return Err((
                StatusCode::FORBIDDEN,
                "Only the host can start the game".into(),
            ));
        }

        (
            game.word_bank_source.clone(),
            game.difficulty,
            game.time_limit.map(|duration| duration.as_secs()),
        )
    };

    // Generate the puzzle grid
    let generated = build_game_grid(&pool, &word_bank_source, difficulty)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let GeneratedGrid {
        grid,
        theme_label_override,
    } = generated;
    let placed_words: Vec<String> = grid
        .placed_words
        .iter()
        .map(|placed| placed.word.clone())
        .collect();

    let start_message = {
        let mut game = room.write().await;

        if game.status != GameStatus::Lobby {
            return Err((StatusCode::CONFLICT, "Game is not in lobby state".into()));
        }

        let player = game
            .players
            .get(&user_uuid)
            .ok_or((StatusCode::FORBIDDEN, "You are not in this game".into()))?;
        if !player.is_host {
            return Err((
                StatusCode::FORBIDDEN,
                "Only the host can start the game".into(),
            ));
        }

        game.grid = Some(grid);
        if let Some(label) = theme_label_override.clone() {
            game.theme = label;
        }
        game.status = GameStatus::InProgress;
        game.finished_reason = None;
        game.started_at = Some(chrono::Utc::now());
        game.touch();

        ServerMessage::GameStarting {
            grid: game.grid.clone().expect("grid just initialized"),
            words_to_find: game
                .grid
                .as_ref()
                .expect("grid just initialized")
                .placed_words
                .iter()
                .map(|pw| pw.word.clone())
                .collect(),
            time_limit,
        }
    };

    repository::update_game_started(&pool, &game_id.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    record_word_bank_usage(&pool, &word_bank_source, &placed_words).await;

    {
        let channel_map = channels.read().await;
        if let Some(tx) = channel_map.get(&game_id) {
            let _ = tx.send(start_message);
            if let Some(room) = get_room_handle(&rooms, game_id).await {
                let game = room.read().await;
                let _ = tx.send(game_state_sync_message(&game));
            }
        }
    }

    if let Some(room) = get_room_handle(&rooms, game_id).await {
        let game = room.read().await.clone();
        persist_game_state_snapshot(&pool, &game).await;
    }

    schedule_game_timeout_watch(pool.clone(), rooms.clone(), channels.clone(), game_id).await;

    Ok(Json(GameResponse {
        id: game_id.to_string(),
    }))
}

async fn solo_game(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<GameResponse>, (StatusCode, String)> {
    let user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    // Solo game: use medium difficulty, random theme
    let game_id = Uuid::new_v4();
    let user_uuid = Uuid::parse_str(&auth_user.user_id).unwrap_or(Uuid::new_v4());
    let theme = parse_word_theme(user.preferred_word_theme.as_deref());
    let difficulty = Difficulty::Medium;
    let theme_key = catalog::theme_key(theme).to_string();
    let word_bank_source = WordBankSource::BuiltIn { theme };

    let generated = build_game_grid(&pool, &word_bank_source, difficulty)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let GeneratedGrid {
        grid,
        theme_label_override,
    } = generated;
    let placed_words: Vec<String> = grid
        .placed_words
        .iter()
        .map(|placed| placed.word.clone())
        .collect();

    let mut players = HashMap::new();
    players.insert(
        user_uuid,
        PlayerState {
            id: user_uuid,
            username: auth_user.username.clone(),
            score: 0,
            is_host: true,
            ready: false,
            highlight_color: user.highlight_color,
            connected: true,
        },
    );

    let game_state = GameState {
        id: game_id,
        grid: Some(grid),
        theme: theme_label_override.unwrap_or(theme_key.clone()),
        word_bank_source: word_bank_source.clone(),
        difficulty,
        players,
        found_words: HashMap::new(),
        status: GameStatus::InProgress,
        finished_reason: None,
        started_at: Some(chrono::Utc::now()),
        time_limit: None,
        max_players: 1,
        revision: 1,
        updated_at: chrono::Utc::now(),
        word_claims: HashMap::new(),
    };

    repository::insert_game(
        &pool,
        &game_id,
        &theme_key,
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

    record_word_bank_usage(&pool, &word_bank_source, &placed_words).await;

    rooms
        .write()
        .await
        .insert(game_id, room_handle(game_state.clone()));
    persist_game_state_snapshot(&pool, &game_state).await;

    Ok(Json(GameResponse {
        id: game_id.to_string(),
    }))
}

async fn build_game_grid(
    pool: &DbPool,
    word_bank_source: &WordBankSource,
    difficulty: Difficulty,
) -> Result<GeneratedGrid, String> {
    let mut last_error = String::from("Failed to generate grid");

    for _ in 0..12 {
        let generated = words_for_game(pool, word_bank_source, difficulty).await?;
        match generate_grid(&generated.words, difficulty) {
            Ok(grid) => {
                return Ok(GeneratedGrid {
                    grid,
                    theme_label_override: generated.theme_label_override,
                })
            }
            Err(error) => last_error = error.to_string(),
        }
    }

    Err(last_error)
}

async fn words_for_game(
    pool: &DbPool,
    word_bank_source: &WordBankSource,
    difficulty: Difficulty,
) -> Result<GeneratedWords, String> {
    match word_bank_source {
        WordBankSource::BuiltIn { theme } => {
            if *theme == WordTheme::RecipeIngredients {
                let selection = crate::puzzles::recipe_ingredients::select_recipe_words(
                    difficulty.word_count(),
                    Some(difficulty.grid_size()),
                )
                .ok_or_else(|| {
                    "Recipe ingredients word bank does not have enough words for this difficulty"
                        .to_string()
                })?;
                if selection.words.len() < difficulty.word_count() {
                    return Err(
                        "Recipe ingredients word bank does not have enough words for this difficulty".into(),
                    );
                }
                return Ok(GeneratedWords {
                    words: selection.words,
                    theme_label_override: Some(format!("Recipe Ingredients: {}", selection.label)),
                });
            }

            let theme_key = catalog::theme_key(*theme).to_string();
            let words = word_banks::built_in_words(*theme, pool).await?;
            let candidates: Vec<String> = words
                .into_iter()
                .filter(|word| word.len() <= difficulty.grid_size())
                .collect();
            let usage = repository::list_builtin_word_bank_word_usage(pool, &theme_key)
                .await
                .map_err(|error| error.to_string())?;
            let usage_by_word: HashMap<String, (i64, Option<String>)> = usage
                .into_iter()
                .map(|entry| (entry.word, (entry.times_used, entry.last_used_at)))
                .collect();

            let selected = word_banks::weighted_word_sample(
                &candidates,
                &usage_by_word,
                difficulty.word_count(),
            );

            if selected.len() < difficulty.word_count() {
                return Err(
                    "Built-in word bank does not have enough words for this difficulty".into(),
                );
            }

            Ok(GeneratedWords {
                words: selected,
                theme_label_override: None,
            })
        }
        WordBankSource::Custom { id } => {
            let bank = repository::get_custom_word_bank_by_id(pool, &id.to_string())
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "Custom word bank not found".to_string())?;

            if bank.status != "ready" {
                return Err("Custom word bank is not ready yet".into());
            }

            let raw_words = bank
                .words_json
                .ok_or_else(|| "Custom word bank has no words".to_string())?;
            let words: Vec<String> = serde_json::from_str(&raw_words)
                .map_err(|error| format!("Custom word bank is invalid: {error}"))?;
            let candidates: Vec<String> = words
                .into_iter()
                .filter(|word| word.len() <= difficulty.grid_size())
                .collect();
            let usage = repository::list_custom_word_bank_word_usage(pool, &id.to_string())
                .await
                .map_err(|error| error.to_string())?;
            let usage_by_word: HashMap<String, (i64, Option<String>)> = usage
                .into_iter()
                .map(|entry| (entry.word, (entry.times_used, entry.last_used_at)))
                .collect();

            let selected = word_banks::weighted_custom_word_sample(
                &candidates,
                &usage_by_word,
                difficulty.word_count(),
            );

            if selected.len() < difficulty.word_count() {
                return Err(
                    "Custom word bank does not have enough words for this difficulty".into(),
                );
            }

            Ok(GeneratedWords {
                words: selected,
                theme_label_override: None,
            })
        }
    }
}

async fn record_word_bank_usage(
    pool: &DbPool,
    word_bank_source: &WordBankSource,
    words: &[String],
) {
    match word_bank_source {
        WordBankSource::BuiltIn { theme } => {
            let theme_key = catalog::theme_key(*theme).to_string();
            if let Err(error) =
                repository::record_builtin_word_bank_usage(pool, &theme_key, words).await
            {
                tracing::warn!(theme_key = %theme_key, error = %error, "failed to record built-in word usage");
                return;
            }

            word_banks::maybe_schedule_builtin_extension(pool.clone(), *theme).await;
        }
        WordBankSource::Custom { id } => {
            if let Err(error) =
                repository::record_custom_word_bank_usage(pool, &id.to_string(), words).await
            {
                tracing::warn!(word_bank_id = %id, error = %error, "failed to record custom word usage");
                return;
            }

            word_banks::maybe_schedule_extension(pool.clone(), id.to_string()).await;
        }
    }
}

struct ResolvedWordBank {
    label: String,
    source: WordBankSource,
}

async fn resolve_requested_word_bank(
    pool: &DbPool,
    requester_user_id: &str,
    theme: Option<&str>,
    word_bank_id: Option<&str>,
) -> Result<ResolvedWordBank, (StatusCode, String)> {
    if let Some(custom_id) = word_bank_id {
        let bank = repository::get_custom_word_bank_by_id(pool, custom_id)
            .await
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Custom word bank not found".into()))?;

        if bank.status != "ready" {
            return Err((
                StatusCode::CONFLICT,
                "Custom word bank is still generating".into(),
            ));
        }

        if bank.user_id != requester_user_id && bank.is_shared == 0 {
            return Err((
                StatusCode::FORBIDDEN,
                "This custom word bank is private to its owner".into(),
            ));
        }

        let label = bank.label.filter(|value| !value.trim().is_empty()).ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Custom word bank is missing a label".into(),
        ))?;
        let id = Uuid::parse_str(custom_id).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid custom word bank id".into(),
            )
        })?;

        return Ok(ResolvedWordBank {
            label,
            source: WordBankSource::Custom { id },
        });
    }

    let theme = theme
        .and_then(catalog::parse_theme_key)
        .unwrap_or(WordTheme::Science);

    Ok(ResolvedWordBank {
        label: catalog::theme_key(theme).to_string(),
        source: WordBankSource::BuiltIn { theme },
    })
}

fn parse_word_theme(value: Option<&str>) -> WordTheme {
    value
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
        .and_then(catalog::parse_theme_key)
        .unwrap_or(WordTheme::Science)
}
