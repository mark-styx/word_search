use axum::{routing::{get, post}, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
}

async fn register() {}
async fn login() {}
async fn me() {}
