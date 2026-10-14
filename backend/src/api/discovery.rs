use axum::{
    extract::Host,
    http::HeaderMap,
    routing::get,
    Json, Router,
};
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
    server_name: String,
    hostname: String,
    port: u16,
    api_url: String,
    frontend_url: String,
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
    Host(host): Host,
    headers: HeaderMap,
    axum::extract::Extension(rooms): axum::extract::Extension<GameRooms>,
) -> Json<InfoResponse> {
    let room_handles: Vec<_> = rooms.read().await.values().cloned().collect();
    let mut active_games = 0;
    let mut players_online = 0;
    for room in room_handles {
        let game = room.read().await;
        if game.status == GameStatus::InProgress {
            active_games += 1;
        }
        players_online += game.players.values().filter(|player| player.connected).count();
    }

    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".into());
    let server_name = format!("Word Search ({hostname})");

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);
    let frontend_port = std::env::var("FRONTEND_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3089);
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("origin")
                .and_then(|value| value.to_str().ok())
                .and_then(|origin| origin.split("://").next())
        })
        .unwrap_or("http");
    let host_without_port = host
        .rsplit_once(':')
        .map(|(value, _)| value)
        .unwrap_or(host.as_str());
    let api_url = format!("{scheme}://{host_without_port}:{port}");
    let frontend_url = std::env::var("PUBLIC_FRONTEND_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("{scheme}://{host_without_port}:{frontend_port}"));

    Json(InfoResponse {
        server_name,
        hostname,
        port,
        api_url,
        frontend_url,
        active_games,
        players_online,
    })
}
