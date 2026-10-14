use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use word_search_backend::api;
use word_search_backend::auth;
use word_search_backend::db::repository;
use word_search_backend::discovery::advertise::MdnsAdvertiser;
use word_search_backend::word_banks;

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
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "./data/wordsearch.db".into());
    let _ = auth::jwt_secret();

    // Initialize database
    let pool = repository::create_pool(&db_path)
        .await
        .expect("Failed to create database pool");

    repository::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Database initialized at {}", db_path);
    word_banks::log_generator_status();

    // Start mDNS advertiser
    let mdns = match MdnsAdvertiser::new(port) {
        Ok(m) => {
            tracing::info!("mDNS advertiser started");
            Some(m)
        }
        Err(e) => {
            tracing::warn!("mDNS advertiser failed to start: {}", e);
            None
        }
    };

    // In-memory game rooms
    let rooms: api::GameRooms = Arc::new(RwLock::new(HashMap::new()));

    let app = api::create_router(pool, rooms);

    let bind_addr = format!("{}:{}", host, port);
    tracing::info!("Server listening on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // Clean up mDNS
    drop(mdns);
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Shutting down...");
}
