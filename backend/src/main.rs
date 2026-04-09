use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use word_search_backend::api;
use word_search_backend::db::repository;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()
        .expect("Invalid PORT");

    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "./data/wordsearch.db".into());

    // Initialize database
    let pool = repository::create_pool(&db_path)
        .await
        .expect("Failed to create database pool");

    repository::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Database initialized at {}", db_path);

    // In-memory game rooms
    let rooms: api::GameRooms = Arc::new(RwLock::new(HashMap::new()));

    let app = api::create_router(pool, rooms);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Shutting down...");
}
