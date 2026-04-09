// REST API route handlers
pub mod auth;
pub mod discovery;
pub mod games;
pub mod stats;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn create_router() -> Router {
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
}
