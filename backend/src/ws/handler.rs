use crate::api::{games, get_room_handle, GameRooms};
use crate::auth;
use crate::db::repository::{self, DbPool};
use crate::game::grid::validate_selection;
use crate::models::{ClientMessage, GameFinishReason, GameStatus, ServerMessage};
use crate::ws::room;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{Duration, Instant, MissedTickBehavior};
use uuid::Uuid;

const CONTESTED_WORD_WINDOW_MS: i64 = 5_000;
const DISCONNECT_GRACE_PERIOD_SECS: u64 = 12;
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const HEARTBEAT_TIMEOUT_SECS: u64 = 15;

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
    let secret = auth::jwt_secret();
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
    pub presence: Arc<RwLock<HashMap<(Uuid, Uuid), usize>>>,
}

enum SelectCellsOutcome {
    Rejected {
        word: String,
        message: &'static str,
        rollback_claim: Option<repository::FoundWordClaimRow>,
    },
    AlreadyFound {
        word: String,
        existing_claim_user_id: Option<Uuid>,
        existing_claimed_at: Option<chrono::DateTime<chrono::Utc>>,
        attempted_at: chrono::DateTime<chrono::Utc>,
    },
    WordFound {
        word: String,
        score: u32,
        score_updates: Vec<(Uuid, u32, usize)>,
        finished: bool,
        game_snapshot: crate::models::GameState,
    },
}

fn parse_claim_row(
    claim: repository::FoundWordClaimRow,
) -> Option<(Uuid, chrono::DateTime<chrono::Utc>)> {
    let player_id = Uuid::parse_str(&claim.user_id).ok()?;
    let claimed_at = chrono::DateTime::parse_from_rfc3339(&claim.found_at)
        .ok()?
        .with_timezone(&chrono::Utc);
    Some((player_id, claimed_at))
}

async fn rollback_claim_result(
    pool: &DbPool,
    game_id: &str,
    word: &str,
    challenger_user_id: Uuid,
    challenger_found_at: &str,
    previous_claim: Option<repository::FoundWordClaimRow>,
) {
    if let Some(previous_claim) = previous_claim {
        let _ = repository::restore_found_word_claim(
            pool,
            &Uuid::new_v4(),
            game_id,
            &previous_claim.user_id,
            word,
            &previous_claim.found_at,
        )
        .await;
    } else {
        let _ = repository::delete_found_word_claim_if_matches(
            pool,
            game_id,
            word,
            &challenger_user_id.to_string(),
            challenger_found_at,
        )
        .await;
    }
}

async fn handle_socket(
    socket: WebSocket,
    state: WsState,
    game_id: Uuid,
    user_id: Uuid,
    _username: String,
) {
    let (mut sender, mut receiver) = socket.split();

    match room::ensure_room_loaded(&state.pool, &state.rooms, game_id).await {
        Ok(true) => {}
        Ok(false) => {
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&ServerMessage::Error {
                        message: "Game not found".into(),
                    })
                    .unwrap_or_default(),
                ))
                .await;
            let _ = sender.close().await;
            return;
        }
        Err(error) => {
            tracing::warn!(game_id = %game_id, error = %error, "failed to load room snapshot");
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&ServerMessage::Error {
                        message: "Failed to load game state".into(),
                    })
                    .unwrap_or_default(),
                ))
                .await;
            let _ = sender.close().await;
            return;
        }
    }

    let Some(room) = get_room_handle(&state.rooms, game_id).await else {
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&ServerMessage::Error {
                    message: "Game not found".into(),
                })
                .unwrap_or_default(),
            ))
            .await;
        let _ = sender.close().await;
        return;
    };

    games::schedule_game_timeout_watch(
        state.pool.clone(),
        state.rooms.clone(),
        state.channels.clone(),
        game_id,
    )
    .await;

    let (initial_messages, reconnect_snapshot) = {
        let mut game = room.write().await;
        if !game.players.contains_key(&user_id) {
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&ServerMessage::Error {
                        message: "You are not in this game".into(),
                    })
                    .unwrap_or_default(),
                ))
                .await;
            let _ = sender.close().await;
            return;
        }

        let mut reconnected = false;
        if let Some(player) = game.players.get_mut(&user_id) {
            if !player.connected {
                player.connected = true;
                reconnected = true;
                game.touch();
            }
        }

        let mut messages = Vec::new();
        if let Some(grid) = &game.grid {
            let words: Vec<String> = grid.placed_words.iter().map(|pw| pw.word.clone()).collect();
            messages.push(ServerMessage::GameStarting {
                grid: grid.clone(),
                words_to_find: words,
                time_limit: game.time_limit.map(|d| d.as_secs()),
            });
        }
        let snapshot = game.clone();
        messages.push(room::game_state_sync_message(&snapshot));
        (messages, reconnected.then_some(snapshot))
    };

    {
        let mut presence = state.presence.write().await;
        *presence.entry((game_id, user_id)).or_insert(0) += 1;
    }

    // Ensure broadcast channel exists for this game
    let tx = room::ensure_room_channel(&state.channels, game_id).await;

    // Subscribe to broadcasts
    let mut rx = tx.subscribe();

    if let Some(snapshot) = reconnect_snapshot {
        room::persist_game_state_snapshot(&state.pool, &snapshot).await;
        let _ = tx.send(room::game_state_sync_message(&snapshot));
    }

    for message in initial_messages {
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&message).unwrap_or_default(),
            ))
            .await;
    }

    let explicit_leave = Arc::new(AtomicBool::new(false));
    let heartbeat_expired = Arc::new(AtomicBool::new(false));
    let last_client_activity = Arc::new(RwLock::new(Instant::now()));

    // Task 1: Forward broadcast messages to this WebSocket
    let tx_clone = tx.clone();
    let last_client_activity_send = last_client_activity.clone();
    let heartbeat_expired_send = heartbeat_expired.clone();
    let mut send_task = tokio::spawn(async move {
        let mut heartbeat = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
        heartbeat.tick().await;

        loop {
            tokio::select! {
                result = rx.recv() => {
                    let Ok(msg) = result else {
                        break;
                    };
                    let text = serde_json::to_string(&msg).unwrap_or_default();
                    if sender.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                _ = heartbeat.tick() => {
                    let last_activity = *last_client_activity_send.read().await;
                    if last_activity.elapsed() >= Duration::from_secs(HEARTBEAT_TIMEOUT_SECS) {
                        heartbeat_expired_send.store(true, Ordering::SeqCst);
                        let _ = sender.send(Message::Close(None)).await;
                        break;
                    }

                    if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
            }
        }
        drop(tx_clone);
    });

    // Task 2: Handle incoming messages from client
    let tx2 = tx.clone();
    let rooms = state.rooms.clone();
    let pool = state.pool.clone();
    let explicit_leave_recv = explicit_leave.clone();
    let last_client_activity_recv = last_client_activity.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            *last_client_activity_recv.write().await = Instant::now();
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
                            if !user_is_in_game(&rooms, game_id, user_id).await {
                                send_select_error(&tx2, "You are not in this game");
                                continue;
                            }
                            let chat_msg = ServerMessage::ChatMessage {
                                player_id: user_id,
                                message,
                                timestamp: chrono::Utc::now(),
                            };
                            let _ = tx2.send(chat_msg);
                        }
                        ClientMessage::Ready => {
                            handle_player_ready(&rooms, &pool, &tx2, game_id, user_id).await;
                        }
                        ClientMessage::LeaveGame => {
                            explicit_leave_recv.store(true, Ordering::SeqCst);
                            break;
                        }
                    }
                }
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {},
        _ = &mut recv_task => {},
    }

    send_task.abort();
    recv_task.abort();

    schedule_disconnect_cleanup(
        state,
        game_id,
        user_id,
        explicit_leave.load(Ordering::SeqCst),
        heartbeat_expired.load(Ordering::SeqCst),
    )
    .await;
}

async fn handle_select_cells(
    rooms: &GameRooms,
    pool: &DbPool,
    tx: &broadcast::Sender<ServerMessage>,
    game_id: Uuid,
    user_id: Uuid,
    cells: Vec<(usize, usize)>,
) {
    let game_id_string = game_id.to_string();
    let attempted_at = chrono::Utc::now();
    let found_at = attempted_at.to_rfc3339();
    let Some(room) = get_room_handle(rooms, game_id).await else {
        return send_select_error(tx, "Game not found");
    };

    let (word, score, total_words) = {
        let game = room.read().await;

        if game.status != GameStatus::InProgress {
            return send_select_error(tx, "Game is not in progress");
        }

        if !game.players.contains_key(&user_id) {
            return send_select_error(tx, "You are not in this game");
        }

        let grid = match &game.grid {
            Some(g) => g.clone(),
            None => return send_select_error(tx, "No grid"),
        };

        let Some(word) = validate_selection(&grid, cells) else {
            return;
        };

        (word.clone(), game.difficulty.score_for_word(&word), grid.placed_words.len())
    };

    let claim_result = match repository::claim_found_word(
        pool,
        &Uuid::new_v4(),
        &game_id_string,
        &user_id.to_string(),
        &word,
        &found_at,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(game_id = %game_id, user_id = %user_id, word, error = %error, "failed to persist found word");
            return send_select_error(tx, "Failed to record word");
        }
    };

    let outcome = {
        let mut game = room.write().await;
        if game.status != GameStatus::InProgress {
            SelectCellsOutcome::Rejected {
                word,
                message: "Game is not in progress",
                rollback_claim: match &claim_result {
                    repository::ClaimFoundWordResult::Won { previous_claim } => {
                        previous_claim.clone()
                    }
                    repository::ClaimFoundWordResult::Lost { .. } => None,
                },
            }
        } else if !game.players.contains_key(&user_id) {
            SelectCellsOutcome::Rejected {
                word,
                message: "You are not in this game",
                rollback_claim: match &claim_result {
                    repository::ClaimFoundWordResult::Won { previous_claim } => {
                        previous_claim.clone()
                    }
                    repository::ClaimFoundWordResult::Lost { .. } => None,
                },
            }
        } else {
            match &claim_result {
                repository::ClaimFoundWordResult::Lost { winning_claim } => {
                    let existing_claim = parse_claim_row(winning_claim.clone());
                    SelectCellsOutcome::AlreadyFound {
                        word,
                        existing_claim_user_id: existing_claim.map(|claim| claim.0),
                        existing_claimed_at: existing_claim.map(|claim| claim.1),
                        attempted_at,
                    }
                }
                repository::ClaimFoundWordResult::Won { previous_claim } => {
                    let mut score_updates = Vec::new();

                    let stale_claim = game.word_claims.get(&word).cloned();
                    if let Some(existing_claim) = &stale_claim {
                        if existing_claim.player_id != user_id {
                            let existing_score =
                                if let Some(existing_player) =
                                    game.players.get_mut(&existing_claim.player_id)
                                {
                                    existing_player.score =
                                        existing_player.score.saturating_sub(score);
                                    existing_player.score
                                } else {
                                    0
                                };
                            let found_count = game
                                .found_words
                                .get_mut(&existing_claim.player_id)
                                .map(|words| {
                                    words.retain(|found_word| found_word != &word);
                                    words.len()
                                })
                                .unwrap_or(0);
                            score_updates.push((
                                existing_claim.player_id,
                                existing_score,
                                found_count,
                            ));
                        }
                    } else if let Some(previous_claim) =
                        previous_claim.clone().and_then(parse_claim_row).filter(|claim| claim.0 != user_id)
                    {
                        let existing_score =
                            if let Some(existing_player) = game.players.get_mut(&previous_claim.0) {
                                existing_player.score = existing_player.score.saturating_sub(score);
                                existing_player.score
                            } else {
                                0
                            };
                        let found_count = game
                            .found_words
                            .get_mut(&previous_claim.0)
                            .map(|words| {
                                words.retain(|found_word| found_word != &word);
                                words.len()
                            })
                            .unwrap_or(0);
                        score_updates.push((
                            previous_claim.0,
                            existing_score,
                            found_count,
                        ));
                    }

                    let already_recorded_for_user = game
                        .found_words
                        .get(&user_id)
                        .is_some_and(|words| words.contains(&word));
                    if !already_recorded_for_user {
                        if let Some(player) = game.players.get_mut(&user_id) {
                            player.score += score;
                        }
                        game.found_words
                            .entry(user_id)
                            .or_insert_with(Vec::new)
                            .push(word.clone());
                    }

                    game.word_claims.insert(
                        word.clone(),
                        crate::models::WordClaim {
                            player_id: user_id,
                            claimed_at: attempted_at,
                        },
                    );
                    game.touch();

                    let player_score = game.players.get(&user_id).map(|p| p.score).unwrap_or(0);
                    let found_count = game.found_words.get(&user_id).map(|v| v.len()).unwrap_or(0);
                    score_updates.push((user_id, player_score, found_count));

                    let all_found: usize = game.found_words.values().map(|v| v.len()).sum();

                    let finished = if all_found >= total_words {
                        games::build_finished_game_update(&mut game, GameFinishReason::Completed)
                            .is_some()
                    } else {
                        false
                    };

                    SelectCellsOutcome::WordFound {
                        word,
                        score,
                        score_updates,
                        finished,
                        game_snapshot: game.clone(),
                    }
                }
            }
        }
    };

    match outcome {
        SelectCellsOutcome::Rejected {
            word,
            message,
            rollback_claim,
        } => {
            rollback_claim_result(
                pool,
                &game_id_string,
                &word,
                user_id,
                &found_at,
                rollback_claim,
            )
            .await;
            send_select_error(tx, message);
        }
        SelectCellsOutcome::AlreadyFound {
            word,
            existing_claim_user_id,
            existing_claimed_at,
            attempted_at,
        } => {
            handle_already_found_word(
                pool,
                tx,
                &game_id_string,
                user_id,
                word,
                attempted_at,
                existing_claim_user_id.zip(existing_claimed_at),
            )
            .await;
        }
        SelectCellsOutcome::WordFound {
            word,
            score,
            score_updates,
            finished,
            game_snapshot,
        } => {
            if let repository::ClaimFoundWordResult::Won {
                previous_claim: Some(previous_claim),
            } = &claim_result
            {
                if let Some((previous_user_id, previous_claimed_at)) =
                    parse_claim_row(previous_claim.clone())
                {
                    let delta_ms = (previous_claimed_at - attempted_at).num_milliseconds();
                    if previous_user_id != user_id && (0..=CONTESTED_WORD_WINDOW_MS).contains(&delta_ms)
                    {
                        let _ = repository::record_contested_word_event(
                            pool,
                            &Uuid::new_v4(),
                            &game_id_string,
                            &word,
                            &user_id.to_string(),
                            &previous_user_id.to_string(),
                            &attempted_at.to_rfc3339(),
                            &previous_claimed_at.to_rfc3339(),
                            delta_ms,
                        )
                        .await;
                    }
                }
            }

            let _ = tx.send(ServerMessage::WordFound {
                player_id: user_id,
                word: word.clone(),
                score,
            });

            for (player_id, score, found_count) in score_updates {
                let _ = tx.send(ServerMessage::PlayerScoreUpdate {
                    player_id,
                    score,
                    found_count,
                });
            }

            if finished {
                let snapshot = game_snapshot.clone();
                let mut rankings: Vec<_> = snapshot.players.values().cloned().collect();
                rankings.sort_by(|a, b| {
                    b.score
                        .cmp(&a.score)
                        .then_with(|| a.username.cmp(&b.username))
                        .then_with(|| a.id.cmp(&b.id))
                });
                let _ = repository::update_game_status(pool, &game_id_string, "finished").await;
                for player in snapshot.players.values() {
                    let words_found = snapshot
                        .found_words
                        .get(&player.id)
                        .map(|words| words.len())
                        .unwrap_or(0) as i64;
                    let rank_position = if snapshot.players.len() > 1 {
                        rankings
                            .iter()
                            .position(|entry| entry.id == player.id)
                            .map(|index| index as i64 + 1)
                    } else {
                        None
                    };
                    let _ = repository::update_participant_stats(
                        pool,
                        &game_id_string,
                        &player.id.to_string(),
                        player.score as i64,
                        words_found,
                        rank_position,
                    )
                    .await;
                    let _ = repository::upsert_user_stats(pool, &player.id.to_string()).await;
                }
                let _ = tx.send(ServerMessage::GameFinished {
                    rankings,
                    reason: GameFinishReason::Completed,
                });
            }

            room::persist_game_state_snapshot(pool, &game_snapshot).await;
            let _ = tx.send(room::game_state_sync_message(&game_snapshot));
        }
    }
}

async fn handle_already_found_word(
    pool: &DbPool,
    tx: &broadcast::Sender<ServerMessage>,
    game_id: &str,
    user_id: Uuid,
    word: String,
    attempted_at: chrono::DateTime<chrono::Utc>,
    existing_claim: Option<(Uuid, chrono::DateTime<chrono::Utc>)>,
) {
    if let Some((existing_player_id, existing_claimed_at)) = existing_claim {
        let delta_ms = (attempted_at - existing_claimed_at).num_milliseconds();
        if existing_player_id != user_id && (0..=CONTESTED_WORD_WINDOW_MS).contains(&delta_ms) {
            let _ = repository::record_contested_word_event(
                pool,
                &Uuid::new_v4(),
                game_id,
                &word,
                &existing_player_id.to_string(),
                &user_id.to_string(),
                &existing_claimed_at.to_rfc3339(),
                &attempted_at.to_rfc3339(),
                delta_ms,
            )
            .await;
        }
    }

    let _ = tx.send(ServerMessage::Error {
        message: format!("Word '{}' already found", word),
    });
}

async fn user_is_in_game(rooms: &GameRooms, game_id: Uuid, user_id: Uuid) -> bool {
    let Some(room) = get_room_handle(rooms, game_id).await else {
        return false;
    };
    let game = room.read().await;
    game.players.contains_key(&user_id)
}

async fn handle_player_ready(
    rooms: &GameRooms,
    pool: &DbPool,
    tx: &broadcast::Sender<ServerMessage>,
    game_id: Uuid,
    user_id: Uuid,
) {
    let Some(room_handle) = get_room_handle(rooms, game_id).await else {
        send_select_error(tx, "Game not found");
        return;
    };

    let snapshot = {
        let mut game = room_handle.write().await;
        if game.status != GameStatus::Lobby {
            send_select_error(tx, "Game is already in progress");
            return;
        }

        let Some(player) = game.players.get_mut(&user_id) else {
            send_select_error(tx, "You are not in this game");
            return;
        };

        if player.ready {
            None
        } else {
            player.ready = true;
            game.touch();
            Some(game.clone())
        }
    };

    let Some(snapshot) = snapshot else {
        let _ = tx.send(ServerMessage::PlayerReady { player_id: user_id });
        return;
    };

    room::persist_game_state_snapshot(pool, &snapshot).await;
    let _ = tx.send(ServerMessage::PlayerReady { player_id: user_id });
    let _ = tx.send(room::game_state_sync_message(&snapshot));
}

async fn schedule_disconnect_cleanup(
    state: WsState,
    game_id: Uuid,
    user_id: Uuid,
    immediate: bool,
    skip_grace_period: bool,
) {
    let should_cleanup_now = {
        let mut presence = state.presence.write().await;
        match presence.get_mut(&(game_id, user_id)) {
            Some(count) if *count > 1 => {
                *count -= 1;
                false
            }
            Some(_) => {
                presence.remove(&(game_id, user_id));
                true
            }
            None => false,
        }
    };

    if !should_cleanup_now {
        prune_channel_if_idle(&state, game_id).await;
        return;
    }

    if immediate {
        cleanup_player_departure(state, game_id, user_id, true).await;
        return;
    }

    if skip_grace_period {
        cleanup_player_departure(state, game_id, user_id, false).await;
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(DISCONNECT_GRACE_PERIOD_SECS)).await;

        let still_absent = {
            let presence = state.presence.read().await;
            !presence.contains_key(&(game_id, user_id))
        };

        if still_absent {
            cleanup_player_departure(state, game_id, user_id, false).await;
        }
    });
}

async fn cleanup_player_departure(state: WsState, game_id: Uuid, user_id: Uuid, _force: bool) {
    let Some(room) = get_room_handle(&state.rooms, game_id).await else {
        prune_channel_if_idle(&state, game_id).await;
        return;
    };

    let (removed_player, remove_room, snapshot, broadcast_left, abandon_game) = {
        let mut game = room.write().await;
        let should_remove_player = game.status == GameStatus::Lobby;

        if should_remove_player {
            let removed_player = game.players.remove(&user_id).is_some();
            if !removed_player {
                (false, false, None, false, false)
            } else {
                game.found_words.remove(&user_id);
                game.word_claims.retain(|_, claim| claim.player_id != user_id);

                if game.players.is_empty() {
                    (true, true, None, true, false)
                } else {
                    if !game.players.values().any(|player| player.is_host) {
                        if let Some(next_host) = game.players.values_mut().next() {
                            next_host.is_host = true;
                        }
                    }
                    game.touch();
                    (true, false, Some(game.clone()), true, false)
                }
            }
        } else if let Some(player) = game.players.get_mut(&user_id) {
            if player.connected {
                player.connected = false;
                game.touch();
                let abandon_game = game.status == GameStatus::InProgress
                    && game.players.values().all(|player| !player.connected);
                (true, false, Some(game.clone()), false, abandon_game)
            } else {
                (false, false, None, false, false)
            }
        } else {
            (false, false, None, false, false)
        }
    };

    if !removed_player {
        prune_channel_if_idle(&state, game_id).await;
        return;
    }

    if remove_room {
        state.rooms.write().await.remove(&game_id);
        let _ = repository::update_game_status(&state.pool, &game_id.to_string(), "finished").await;
        let mut channels = state.channels.write().await;
        channels.remove(&game_id);
        return;
    }

    if abandon_game {
        let _ = games::finalize_game(
            &state.pool,
            &state.rooms,
            &state.channels,
            game_id,
            GameFinishReason::Abandoned,
        )
        .await;
        prune_channel_if_idle(&state, game_id).await;
        return;
    }

    if let Some(game) = snapshot {
        room::persist_game_state_snapshot(&state.pool, &game).await;

        let tx = {
            let channels = state.channels.read().await;
            channels.get(&game_id).cloned()
        };
        if let Some(tx) = tx {
            if broadcast_left {
                let _ = tx.send(ServerMessage::PlayerLeft { player_id: user_id });
            }
            let _ = tx.send(room::game_state_sync_message(&game));
        }
    }

    prune_channel_if_idle(&state, game_id).await;
}

async fn prune_channel_if_idle(state: &WsState, game_id: Uuid) {
    let _ = room::prune_channel_if_idle(&state.channels, game_id).await;
}

fn send_select_error(tx: &broadcast::Sender<ServerMessage>, message: &str) {
    let _ = tx.send(ServerMessage::Error {
        message: message.into(),
    });
}
