use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::api::GameRooms;
use crate::models::GameStatus;

#[derive(Serialize)]
struct PingResponse {
    service: String,
    version: String,
}

#[derive(Serialize)]
struct InfoResponse {
    hostname: String,
    port: u16,
    active_games: usize,
    players_online: usize,
}

pub fn routes() -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/info", get(info))
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        service: "word-search".into(),
        version: "1.0".into(),
    })
}

async fn info(
    axum::extract::Extension(rooms): axum::extract::Extension<GameRooms>,
) -> Json<InfoResponse> {
    let rooms_guard = rooms.read().await;
    let active_games = rooms_guard.values().filter(|g| g.status == GameStatus::InProgress).count();
    let players_online: usize = rooms_guard.values().map(|g| g.players.len()).sum();

    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".into());

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    Json(InfoResponse {
        hostname,
        port,
        active_games,
        players_online,
    })
}
