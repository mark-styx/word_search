// REST API route handlers
pub mod auth;
pub mod discovery;
pub mod games;
pub mod stats;
pub mod word_banks;

use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::db::repository::DbPool;
use crate::models::{GameState, ServerMessage};
use crate::ws::handler::WsState;

/// Shared in-memory game state.
pub type GameRoom = Arc<RwLock<GameState>>;
pub type GameRooms = Arc<RwLock<HashMap<Uuid, GameRoom>>>;

/// Broadcast channels per game room.
pub type GameChannels = Arc<RwLock<HashMap<Uuid, broadcast::Sender<ServerMessage>>>>;

pub fn room_handle(game: GameState) -> GameRoom {
    Arc::new(RwLock::new(game))
}

pub async fn get_room_handle(rooms: &GameRooms, game_id: Uuid) -> Option<GameRoom> {
    rooms.read().await.get(&game_id).cloned()
}

pub fn create_router(pool: DbPool, rooms: GameRooms) -> Router {
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let channels: GameChannels = Arc::new(RwLock::new(HashMap::new()));
    let ws_state = WsState {
        pool: pool.clone(),
        rooms: rooms.clone(),
        channels: channels.clone(),
        presence: Arc::new(RwLock::new(HashMap::new())),
    };

    let auth_mw = axum::middleware::from_fn(crate::auth::auth_middleware);

    // Stateful routes (WebSocket)
    let ws_routes = Router::new()
        .route("/ws", axum::routing::get(crate::ws::handler::ws_upgrade))
        .with_state(ws_state);

    // Each prefix gets one nest() call; public and protected routes are merged
    // with auth middleware applied only to protected branches.
    let games_router = Router::new()
        .merge(games::public_routes())
        .merge(games::protected_routes().layer(auth_mw.clone()));

    let auth_router = Router::new()
        .merge(auth::routes())
        .merge(auth::protected_routes().layer(auth_mw.clone()));

    let stats_router = Router::new()
        .merge(stats::routes())
        .merge(stats::protected_routes().layer(auth_mw.clone()));

    let word_banks_router =
        Router::new().merge(word_banks::protected_routes().layer(auth_mw.clone()));

    let api_routes = Router::new()
        .nest("/api/auth", auth_router)
        .nest("/api/discover", discovery::routes())
        .nest("/api/games", games_router)
        .nest("/api/stats", stats_router)
        .nest("/api/word-banks", word_banks_router)
        .layer(axum::extract::Extension(pool))
        .layer(axum::extract::Extension(channels))
        .layer(axum::extract::Extension(rooms));

    Router::new()
        .merge(api_routes)
        .merge(ws_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
