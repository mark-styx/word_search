use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/leaderboard", get(leaderboard))
        .route("/me", get(my_stats))
        .route("/me/history", get(my_history))
}

async fn leaderboard() {}
async fn my_stats() {}
async fn my_history() {}
