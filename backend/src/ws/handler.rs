use crate::api::GameRooms;
use crate::auth;
use crate::db::repository::{self, DbPool};
use crate::game::grid::validate_selection;
use crate::models::{ClientMessage, GameStatus, PlayerState, ServerMessage};
use axum::{
    extract::{Query, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WsParams {
    token: String,
    game_id: String,
}

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<WsState>,
) -> impl IntoResponse {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me".into());
    let claims = match auth::verify_jwt(&params.token, &secret) {
        Ok(c) => c,
        Err(_) => return (axum::http::StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };

    let game_id = match Uuid::parse_str(&params.game_id) {
        Ok(id) => id,
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Invalid game ID").into_response(),
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (axum::http::StatusCode::UNAUTHORIZED, "Invalid user ID").into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, game_id, user_id, claims.username))
}

#[derive(Clone)]
pub struct WsState {
    pub pool: DbPool,
    pub rooms: GameRooms,
    pub channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<ServerMessage>>>>,
}

async fn handle_socket(
    socket: WebSocket,
    state: WsState,
    game_id: Uuid,
    user_id: Uuid,
    username: String,
) {
    let (mut sender, mut receiver) = socket.split();

    // Ensure broadcast channel exists for this game
    let tx = {
        let mut channels = state.channels.write().await;
        channels
            .entry(game_id)
            .or_insert_with(|| broadcast::channel(32).0)
            .clone()
    };

    // Subscribe to broadcasts
    let mut rx = tx.subscribe();

    // Send current game state (grid + words) if game is in progress
    {
        let rooms = state.rooms.read().await;
        if let Some(game) = rooms.get(&game_id) {
            if let Some(grid) = &game.grid {
                let words: Vec<String> = grid.placed_words.iter().map(|pw| pw.word.clone()).collect();
                let msg = ServerMessage::GameStarting {
                    grid: grid.clone(),
                    words_to_find: words,
                    time_limit: game.time_limit.map(|d| d.as_secs()),
                };
                let _ = sender.send(Message::Text(serde_json::to_string(&msg).unwrap_or_default())).await;
            }
        }
    }

    // Broadcast player joined
    let player_state = PlayerState {
        id: user_id,
        username: username.clone(),
        score: 0,
        is_host: false,
    };
    let _ = tx.send(ServerMessage::PlayerJoined { player: player_state });

    // Task 1: Forward broadcast messages to this WebSocket
    let tx_clone = tx.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
        drop(tx_clone);
    });

    // Task 2: Handle incoming messages from client
    let tx2 = tx.clone();
    let rooms = state.rooms.clone();
    let pool = state.pool.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let client_msg: ClientMessage = match serde_json::from_str(&text) {
                        Ok(m) => m,
                        Err(_) => {
                            let _ = tx2.send(ServerMessage::Error {
                                message: "Invalid message format".into(),
                            });
                            continue;
                        }
                    };

                    match client_msg {
                        ClientMessage::SelectCells { cells } => {
                            handle_select_cells(&rooms, &pool, &tx2, game_id, user_id, cells).await;
                        }
                        ClientMessage::Chat { message } => {
                            let chat_msg = ServerMessage::ChatMessage {
                                player_id: user_id,
                                message,
                                timestamp: chrono::Utc::now(),
                            };
                            let _ = tx2.send(chat_msg);
                        }
                        ClientMessage::Ready => {
                            let _ = tx2.send(ServerMessage::PlayerReady { player_id: user_id });
                        }
                        ClientMessage::LeaveGame => {
                            let _ = tx2.send(ServerMessage::PlayerLeft { player_id: user_id });
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Cleanup: broadcast player left
    let _ = tx.send(ServerMessage::PlayerLeft { player_id: user_id });
}

async fn handle_select_cells(
    rooms: &GameRooms,
    pool: &DbPool,
    tx: &broadcast::Sender<ServerMessage>,
    game_id: Uuid,
    user_id: Uuid,
    cells: Vec<(usize, usize)>,
) {
    let mut rooms = rooms.write().await;
    let game = match rooms.get_mut(&game_id) {
        Some(g) => g,
        None => {
            let _ = tx.send(ServerMessage::Error { message: "Game not found".into() });
            return;
        }
    };

    if game.status != GameStatus::InProgress {
        let _ = tx.send(ServerMessage::Error { message: "Game is not in progress".into() });
        return;
    }

    let grid = match &game.grid {
        Some(g) => g.clone(),
        None => {
            let _ = tx.send(ServerMessage::Error { message: "No grid".into() });
            return;
        }
    };

    if let Some(word) = validate_selection(&grid, cells) {
        // Check if word already found by anyone
        let already_found = game
            .found_words
            .values()
            .any(|words| words.contains(&word));

        if already_found {
            let _ = tx.send(ServerMessage::Error { message: format!("Word '{}' already found", word) });
            return;
        }

        // Score = word length × difficulty multiplier
        let score = (word.len() as u32) * game.difficulty.score_multiplier();

        // Update player score
        if let Some(player) = game.players.get_mut(&user_id) {
            player.score += score;
        }

        // Record found word
        game.found_words
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(word.clone());

        let player_score = game.players.get(&user_id).map(|p| p.score).unwrap_or(0);
        let found_count = game.found_words.get(&user_id).map(|v| v.len()).unwrap_or(0);

        // Persist found word
        let _ = repository::insert_found_word(pool, &Uuid::new_v4(), &game_id.to_string(), &user_id.to_string(), &word).await;

        // Broadcast word found
        let _ = tx.send(ServerMessage::WordFound {
            player_id: user_id,
            word: word.clone(),
            score,
        });

        let _ = tx.send(ServerMessage::PlayerScoreUpdate {
            player_id: user_id,
            score: player_score,
            found_count,
        });

        // Check if all words found
        let total_words = grid.placed_words.len();
        let all_found: usize = game.found_words.values().map(|v| v.len()).sum();
        if all_found >= total_words {
            // Game finished
            game.status = GameStatus::Finished;

            let mut rankings: Vec<PlayerState> = game.players.values().cloned().collect();
            rankings.sort_by(|a, b| b.score.cmp(&a.score));

            // Persist results
            let _ = repository::update_game_status(pool, &game_id.to_string(), "finished").await;
            for (i, player) in rankings.iter().enumerate() {
                let rank = if rankings.len() > 1 { Some(i as i64 + 1) } else { None };
                let fc = game.found_words.get(&player.id).map(|v| v.len()).unwrap_or(0) as i64;
                let _ = repository::update_participant_stats(
                    pool, &game_id.to_string(), &player.id.to_string(),
                    player.score as i64, fc, rank,
                ).await;
                let _ = repository::upsert_user_stats(pool, &player.id.to_string()).await;
            }

            let _ = tx.send(ServerMessage::GameFinished { rankings });
        }
    }
}
