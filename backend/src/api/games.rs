use axum::{routing::{get, post}, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/", post(create_game))
        .route("/", get(list_games))
        .route("/:id", get(get_game))
        .route("/:id/join", post(join_game))
        .route("/:id/start", post(start_game))
        .route("/solo", get(solo_game))
}

async fn create_game() {}
async fn list_games() {}
async fn get_game() {}
async fn join_game() {}
async fn start_game() {}
async fn solo_game() {}
