// REST API route handlers
pub mod auth;
pub mod discovery;
pub mod games;
pub mod stats;

use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::repository::DbPool;
use crate::models::GameState;

/// Shared in-memory game state.
pub type GameRooms = Arc<RwLock<HashMap<Uuid, GameState>>>;

pub fn create_router(pool: DbPool, rooms: GameRooms) -> Router {
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/games", games::routes())
        .nest("/api/stats", stats::routes())
        .nest("/api/discover", discovery::routes())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::extract::Extension(pool))
        .layer(axum::extract::Extension(rooms))
}
