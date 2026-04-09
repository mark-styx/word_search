// REST API route handlers
pub mod auth;
pub mod discovery;
pub mod games;
pub mod stats;

use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::db::repository::DbPool;
use crate::models::{GameState, ServerMessage};
use crate::ws::handler::WsState;

/// Shared in-memory game state.
pub type GameRooms = Arc<RwLock<HashMap<Uuid, GameState>>>;

/// Broadcast channels per game room.
pub type GameChannels = Arc<RwLock<HashMap<Uuid, broadcast::Sender<ServerMessage>>>>;

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
        channels,
    };

    // Stateful routes (WebSocket)
    let ws_routes = Router::new()
        .route("/ws", axum::routing::get(crate::ws::handler::ws_upgrade))
        .with_state(ws_state);

    // Extension-based routes (REST API)
    let public_routes = Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/discover", discovery::routes())
        .nest("/api/games", games::public_routes());

    let protected_routes = Router::new()
        .nest("/api/games", games::protected_routes())
        .nest("/api/stats", stats::routes())
        .layer(axum::middleware::from_fn(crate::auth::auth_middleware));

    let api_routes = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(axum::extract::Extension(pool))
        .layer(axum::extract::Extension(rooms));

    Router::new()
        .merge(api_routes)
        .merge(ws_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
