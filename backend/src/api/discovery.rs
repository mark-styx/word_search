use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/info", get(info))
}

async fn ping() {}
async fn info() {}
