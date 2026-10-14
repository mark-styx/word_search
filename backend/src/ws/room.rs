use std::collections::HashMap;

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::api::{room_handle, GameChannels, GameRooms};
use crate::db::repository::{self, DbPool};
use crate::models::{GameState, ServerMessage};

pub fn game_state_sync_message(game: &GameState) -> ServerMessage {
    ServerMessage::GameStateSync {
        game: serde_json::to_value(crate::api::games::game_detail_response_from_state(game))
            .unwrap_or(serde_json::Value::Null),
    }
}

pub async fn persist_game_state_snapshot(pool: &DbPool, game: &GameState) {
    match serde_json::to_string(game) {
        Ok(snapshot) => {
            if let Err(error) =
                repository::upsert_game_state_snapshot(pool, &game.id.to_string(), &snapshot).await
            {
                tracing::warn!(game_id = %game.id, error = %error, "failed to persist game snapshot");
            }
        }
        Err(error) => {
            tracing::warn!(game_id = %game.id, error = %error, "failed to serialize game snapshot");
        }
    }
}

pub async fn ensure_room_loaded(
    pool: &DbPool,
    rooms: &GameRooms,
    game_id: Uuid,
) -> Result<bool, String> {
    {
        let rooms_guard = rooms.read().await;
        if rooms_guard.contains_key(&game_id) {
            return Ok(true);
        }
    }

    let snapshot = repository::get_game_state_snapshot(pool, &game_id.to_string())
        .await
        .map_err(|error| error.to_string())?;
    let Some(snapshot) = snapshot else {
        return Ok(false);
    };
    let game = deserialize_game_snapshot(&snapshot)?;

    let mut rooms_guard = rooms.write().await;
    rooms_guard
        .entry(game_id)
        .or_insert_with(|| room_handle(game));
    Ok(true)
}

pub async fn warm_active_rooms_from_snapshots(
    pool: &DbPool,
    rooms: &GameRooms,
) -> Result<(), String> {
    let snapshots = repository::list_active_game_state_snapshots(pool)
        .await
        .map_err(|error| error.to_string())?;

    if snapshots.is_empty() {
        return Ok(());
    }

    let mut parsed = Vec::new();
    for snapshot in snapshots {
        parsed.push(deserialize_game_snapshot(&snapshot)?);
    }

    let mut rooms_guard = rooms.write().await;
    for game in parsed {
        rooms_guard.entry(game.id).or_insert_with(|| room_handle(game));
    }

    Ok(())
}

pub async fn get_room_channel(
    channels: &GameChannels,
    game_id: Uuid,
) -> Option<broadcast::Sender<ServerMessage>> {
    channels.read().await.get(&game_id).cloned()
}

pub async fn ensure_room_channel(
    channels: &GameChannels,
    game_id: Uuid,
) -> broadcast::Sender<ServerMessage> {
    let mut channel_map = channels.write().await;
    channel_map
        .entry(game_id)
        .or_insert_with(|| broadcast::channel(32).0)
        .clone()
}

pub async fn broadcast_room_state(channels: &GameChannels, game: &GameState) {
    if let Some(tx) = get_room_channel(channels, game.id).await {
        let _ = tx.send(game_state_sync_message(game));
    }
}

pub async fn prune_channel_if_idle(
    channels: &GameChannels,
    game_id: Uuid,
) -> Option<broadcast::Sender<ServerMessage>> {
    let mut channel_map = channels.write().await;
    if let Some(tx) = channel_map.get(&game_id) {
        if tx.receiver_count() == 0 {
            return channel_map.remove(&game_id);
        }
    }
    None
}

pub async fn remove_room_and_channel(
    rooms: &GameRooms,
    channels: &GameChannels,
    game_id: Uuid,
) {
    rooms.write().await.remove(&game_id);
    channels.write().await.remove(&game_id);
}

fn deserialize_game_snapshot(snapshot: &str) -> Result<GameState, String> {
    serde_json::from_str(snapshot).map_err(|error| format!("invalid persisted game snapshot: {error}"))
}

pub fn empty_presence() -> HashMap<(Uuid, Uuid), usize> {
    HashMap::new()
}
