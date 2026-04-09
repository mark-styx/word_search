use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;
use word_search_backend::api::{self, GameRooms};
use word_search_backend::db::repository::{self, DbPool};

/// Helper: create in-memory DB pool with migrations.
async fn test_pool() -> DbPool {
    let pool = repository::create_pool(":memory:").await.unwrap();
    repository::run_migrations(&pool).await.unwrap();
    pool
}

/// Helper: build a fresh Router for each request (stateless).
fn build_app(pool: DbPool) -> axum::Router {
    let rooms: GameRooms = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    api::create_router(pool, rooms)
}

/// Helper: build a router sharing the same GameRooms across calls.
fn build_app_shared(pool: DbPool, rooms: GameRooms) -> axum::Router {
    api::create_router(pool, rooms)
}

fn auth_header(token: &str) -> String {
    format!("Bearer {}", token)
}

/// Helper: register using a shared pool, return token.
async fn register(pool: &DbPool, username: &str) -> String {
    let app = build_app(pool.clone());
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": username, "password": "test123456" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    v["token"].as_str().unwrap().to_string()
}

/// Helper: parse JSON body from a response.
async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 65536).await.unwrap();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            let text = String::from_utf8_lossy(&bytes);
            panic!("Failed to parse JSON: {}. Body: {}", e, text);
        })
    }
}

// ===========================================================================
// -- Auth tests --------------------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn register_success() {
    let pool = test_pool().await;
    let token = register(&pool, "alice").await;
    assert!(!token.is_empty());
}

#[tokio::test]
async fn register_duplicate_username() {
    let pool = test_pool().await;
    register(&pool, "bob").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "bob", "password": "test123456" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn register_short_username() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "ab", "password": "test123456" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_short_password() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "charlie", "password": "12345" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_success() {
    let pool = test_pool().await;
    register(&pool, "dave").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "dave", "password": "test123456" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    assert!(v["token"].is_string());
    assert_eq!(v["user"]["username"], "dave");
}

#[tokio::test]
async fn login_wrong_password() {
    let pool = test_pool().await;
    register(&pool, "eve").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "eve", "password": "wrong-password" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_nonexistent_user() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "nobody", "password": "test123456" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_returns_user_info() {
    let pool = test_pool().await;
    let token = register(&pool, "frank_me").await;

    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/auth/me")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["username"], "frank_me");
}

#[tokio::test]
async fn me_without_token_returns_unauthorized() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/auth/me")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// -- Auth middleware tests ---------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn protected_route_without_token_returns_401() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "theme": "science", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_route_with_invalid_token_returns_401() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", "Bearer invalid.token.here")
        .body(Body::from(
            json!({ "theme": "science", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stats_endpoints_require_auth() {
    let pool = test_pool().await;

    // /api/stats/me
    let app = build_app(pool.clone());
    let req = Request::builder()
        .uri("/api/stats/me")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // /api/stats/me/history
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/me/history")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// -- Game creation tests -----------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn create_game_success() {
    let pool = test_pool().await;
    let token = register(&pool, "frank").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&token))
        .body(Body::from(
            json!({ "theme": "science", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let v = body_json(resp).await;
    assert!(v["id"].is_string());
}

#[tokio::test]
async fn create_game_all_themes_and_difficulties() {
    let themes = vec![
        "animals", "sports", "food", "movies", "science",
        "geography", "technology", "music", "history", "nature",
    ];
    let diffs = vec!["easy", "medium", "hard", "expert"];

    for theme in &themes {
        for diff in &diffs {
            let pool = test_pool().await;
            let username = format!("t{}_{}", theme, diff);
            let token = register(&pool, &username).await;

            let app = build_app(pool);
            let req = Request::builder()
                .method("POST")
                .uri("/api/games")
                .header("content-type", "application/json")
                .header("authorization", auth_header(&token))
                .body(Body::from(
                    json!({ "theme": theme, "difficulty": diff }).to_string(),
                ))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::CREATED,
                "Failed for theme={} diff={}",
                theme,
                diff
            );
        }
    }
}

#[tokio::test]
async fn create_game_with_time_limit() {
    let pool = test_pool().await;
    let token = register(&pool, "timed_host").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&token))
        .body(Body::from(
            json!({ "theme": "animals", "difficulty": "medium", "time_limit_secs": 120, "max_players": 4 }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn list_games_empty() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/games")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn get_game_not_found() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/games/00000000-0000-0000-0000-000000000000")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_game_invalid_uuid() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/games/not-a-uuid")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ===========================================================================
// -- Full game lifecycle: create -> get -> join -> start ---------------------
// ===========================================================================

#[tokio::test]
async fn game_lifecycle_create_get_join_start() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    // Register host and second player
    let host_token = register(&pool, "host_lifecycle").await;
    let player_token = register(&pool, "player_lifecycle").await;

    // Host creates a game
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&host_token))
        .body(Body::from(
            json!({ "theme": "animals", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let game_id = body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get game details (public route)
    let req = Request::builder()
        .uri(&format!("/api/games/{}", game_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = body_json(resp).await;
    assert_eq!(detail["status"], "lobby");
    assert_eq!(detail["theme"], "animals");
    assert_eq!(detail["players"].as_array().unwrap().len(), 1);

    // Second player joins
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/join", game_id))
        .header("authorization", auth_header(&player_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Get game again, should have 2 players
    let req = Request::builder()
        .uri(&format!("/api/games/{}", game_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let detail = body_json(resp).await;
    assert_eq!(detail["players"].as_array().unwrap().len(), 2);

    // Host starts the game
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/start", game_id))
        .header("authorization", auth_header(&host_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "Start game failed: {:?}", body_json(resp).await);

    // Game should now be in progress with a grid
    let req = Request::builder()
        .uri(&format!("/api/games/{}", game_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let detail = body_json(resp).await;
    assert_eq!(detail["status"], "in_progress", "Actual: {:?}", detail);
}

#[tokio::test]
async fn join_game_already_in_game() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    let host_token = register(&pool, "host_double").await;

    // Create game
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&host_token))
        .body(Body::from(
            json!({ "theme": "animals", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Host tries to join their own game (already in it) - idempotent, returns 200
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/join", game_id))
        .header("authorization", auth_header(&host_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn start_game_non_host_forbidden() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    let host_token = register(&pool, "host_only").await;
    let player_token = register(&pool, "player_cannot_start").await;

    // Create and join
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&host_token))
        .body(
            Body::from(
                json!({ "theme": "animals", "difficulty": "easy" }).to_string(),
            ),
        )
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/join", game_id))
        .header("authorization", auth_header(&player_token))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // Non-host tries to start
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/start", game_id))
        .header("authorization", auth_header(&player_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn start_game_already_started() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    let token = register(&pool, "host_started").await;

    // Create and start
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&token))
        .body(
            Body::from(
                json!({ "theme": "food", "difficulty": "medium" }).to_string(),
            ),
        )
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/start", game_id))
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "First start failed: {:?}", body_json(resp).await);

    // Try to start again
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/start", game_id))
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn join_game_not_in_lobby() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    let host_token = register(&pool, "host_nl").await;
    let late_token = register(&pool, "late_joiner").await;

    // Create and start
    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&host_token))
        .body(
            Body::from(
                json!({ "theme": "nature", "difficulty": "medium" }).to_string(),
            ),
        )
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Start the game
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/start", game_id))
        .header("authorization", auth_header(&host_token))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // Late joiner tries to join an in-progress game
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/join", game_id))
        .header("authorization", auth_header(&late_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn join_nonexistent_game() {
    let pool = test_pool().await;
    let token = register(&pool, "joiner_ghost").await;
    let app = build_app(pool);

    let req = Request::builder()
        .method("POST")
        .uri("/api/games/00000000-0000-0000-0000-000000000000/join")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ===========================================================================
// -- Solo game tests ---------------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn solo_game_creates_and_starts() {
    let pool = test_pool().await;
    let token = register(&pool, "solo_player").await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    // Solo game uses Medium/Science which can occasionally fail on word placement.
    // Retry up to 5 times if it returns 500.
    let game_id = {
        let mut gid = None;
        for _ in 0..5 {
            let req = Request::builder()
                .uri("/api/games/solo")
                .header("authorization", auth_header(&token))
                .body(Body::empty())
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            if resp.status() == StatusCode::OK {
                let v = body_json(resp).await;
                gid = Some(v["id"].as_str().unwrap().to_string());
                break;
            }
        }
        gid.expect("Solo game failed after 5 retries")
    };

    // Verify game is in progress
    let req = Request::builder()
        .uri(&format!("/api/games/{}", game_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let detail = body_json(resp).await;
    assert_eq!(detail["status"], "in_progress", "Actual: {:?}", detail);
    assert_eq!(detail["max_players"], 1);
}

#[tokio::test]
async fn solo_game_requires_auth() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/games/solo")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// -- List games / lobby ------------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn list_games_shows_lobby_games() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    let token1 = register(&pool, "lobby_host1").await;
    let token2 = register(&pool, "lobby_host2").await;

    // Create 2 games
    for token in &[&token1, &token2] {
        let req = Request::builder()
            .method("POST")
            .uri("/api/games")
            .header("content-type", "application/json")
            .header("authorization", auth_header(token))
            .body(
                Body::from(
                    json!({ "theme": "animals", "difficulty": "medium" }).to_string(),
                ),
            )
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List games
    let req = Request::builder()
        .uri("/api/games")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let list = body_json(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
}

// ===========================================================================
// -- Discovery tests ---------------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn ping_returns_service_info() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/discover/ping")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["service"], "word-search");
    assert_eq!(v["version"], "1.0");
}

#[tokio::test]
async fn info_returns_server_info() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/discover/info")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v["hostname"].is_string());
    assert!(v["port"].is_number());
    assert!(v["active_games"].is_number());
    assert!(v["players_online"].is_number());
}

// ===========================================================================
// -- Stats tests -------------------------------------------------------------
// ===========================================================================

#[tokio::test]
async fn leaderboard_empty() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/leaderboard")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn my_stats_not_found() {
    let pool = test_pool().await;
    let token = register(&pool, "nostats_user").await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/me")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn my_history_empty() {
    let pool = test_pool().await;
    let token = register(&pool, "nohistory_user").await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/me/history")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn leaderboard_with_pagination() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/leaderboard?page=2&limit=5")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ===========================================================================
// -- WebSocket integration tests ---------------------------------------------
// ===========================================================================

mod ws_tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    /// Spin up a real HTTP server on a random port, return (base_url, rooms, pool, shutdown_tx).
    async fn start_server() -> (
        String,
        GameRooms,
        DbPool,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let pool = test_pool().await;
        let rooms: GameRooms = std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ));
        let app = build_app_shared(pool.clone(), rooms.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let base = format!("http://127.0.0.1:{}", addr.port());

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });

        // Give server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        (base, rooms, pool, tx)
    }

    /// Connect a WebSocket client to a game.
    async fn connect_ws(
        base: &str,
        token: &str,
        game_id: &str,
    ) -> (
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) {
        let ws_url = base.replace("http://", "ws://");
        let url = format!("{}/ws?token={}&game_id={}", ws_url, token, game_id);
        let (stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .unwrap();
        stream.split()
    }

    /// Read the next text message from a WebSocket stream.
    async fn next_text(
        rx: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> serde_json::Value {
        let timeout = std::time::Duration::from_secs(5);
        let msg = tokio::time::timeout(timeout, rx.next())
            .await
            .expect("Timeout waiting for WS message")
            .expect("WS stream ended")
            .expect("WS error");
        match msg {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("Expected text message, got: {:?}", other),
        }
    }

    /// Helper: create a solo game via HTTP and return the game ID. Retries on 500.
    async fn create_solo(base: &str, token: &str) -> String {
        let client = reqwest::Client::new();
        for _ in 0..5 {
            let resp = client
                .get(format!("{}/api/games/solo", base))
                .header("authorization", auth_header(token))
                .send()
                .await
                .unwrap();
            if resp.status() == StatusCode::OK {
                return resp.json::<serde_json::Value>()
                    .await
                    .unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_string();
            }
        }
        panic!("Solo game creation failed after 5 retries");
    }

    /// Get cells JSON for a placed word.
    fn cells_for_word(
        pw: &word_search_backend::models::PlacedWord,
    ) -> Vec<serde_json::Value> {
        let (dr, dc) = pw.direction.delta();
        (0..pw.word.len())
            .map(|i| {
                let r = (pw.start.0 as isize + dr * i as isize) as usize;
                let c = (pw.start.1 as isize + dc * i as isize) as usize;
                json!([r, c])
            })
            .collect()
    }

    #[tokio::test]
    async fn ws_connect_and_receive_game_starting() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_host").await;

        let game_id = create_solo(&base, &token).await;

        // Connect WebSocket
        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        // Should receive game_starting message
        let msg = next_text(&mut rx).await;
        assert_eq!(msg["type"], "game_starting");
        assert!(msg["grid"].is_object());
        assert!(msg["words_to_find"].is_array());

        // Also receive player_joined from our own connection
        let msg2 = next_text(&mut rx).await;
        assert_eq!(msg2["type"], "player_joined");

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_invalid_token_rejected() {
        let (base, _rooms, pool, shutdown) = start_server().await;

        let ws_url = base.replace("http://", "ws://");
        let url = format!(
            "{}/ws?token=invalid&game_id=00000000-0000-0000-0000-000000000000",
            ws_url
        );

        let result = tokio_tungstenite::connect_async(&url).await;
        // The server should reject the connection
        assert!(
            result.is_err(),
            "Connection with invalid token should fail"
        );

        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_select_cells_and_find_word() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_finder").await;

        let game_id = create_solo(&base, &token).await;

        // Connect WS
        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        // Skip game_starting and player_joined
        let _starting = next_text(&mut rx).await;
        let _joined = next_text(&mut rx).await;

        // Get grid info to find a valid word selection
        let rooms_guard = rooms.read().await;
        let game = rooms_guard
            .values()
            .find(|g| g.id.to_string() == game_id)
            .unwrap();
        let grid = game.grid.as_ref().unwrap();
        let first_word = &grid.placed_words[0];
        let cells = cells_for_word(first_word);
        let word_text = first_word.word.clone();
        drop(rooms_guard);

        // Send select_cells
        let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        // Should receive word_found
        let found = next_text(&mut rx).await;
        assert_eq!(found["type"], "word_found");
        assert_eq!(found["word"], word_text);

        // Should receive player_score_update
        let score_update = next_text(&mut rx).await;
        assert_eq!(score_update["type"], "player_score_update");

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_find_all_words_finishes_game() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_finisher").await;

        let game_id = create_solo(&base, &token).await;

        // Connect WS
        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        // Skip initial messages
        let _starting = next_text(&mut rx).await;
        let _joined = next_text(&mut rx).await;

        // Get all placed words
        let rooms_guard = rooms.read().await;
        let game = rooms_guard
            .values()
            .find(|g| g.id.to_string() == game_id)
            .unwrap();
        let grid = game.grid.as_ref().unwrap();
        let words_info: Vec<_> = grid
            .placed_words
            .iter()
            .map(|pw| (pw.word.clone(), cells_for_word(pw)))
            .collect();
        drop(rooms_guard);

        // Find each word
        for (i, (_word, cells)) in words_info.iter().enumerate() {
            let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
            tx.send(Message::Text(msg)).await.unwrap();

            // word_found
            let found = next_text(&mut rx).await;
            assert_eq!(
                found["type"],
                "word_found",
                "Expected word_found for word #{}, got: {:?}",
                i,
                found
            );

            // player_score_update
            let score = next_text(&mut rx).await;
            assert_eq!(score["type"], "player_score_update");

            // If this was the last word, we also get game_finished
            if i == words_info.len() - 1 {
                let finished = next_text(&mut rx).await;
                assert_eq!(finished["type"], "game_finished");
                let rankings = finished["rankings"].as_array().unwrap();
                assert_eq!(rankings.len(), 1);
                assert!(rankings[0]["score"].as_u64().unwrap() > 0);
            }
        }

        // Verify game state is finished
        let rooms_guard = rooms.read().await;
        let game = rooms_guard
            .values()
            .find(|g| g.id.to_string() == game_id)
            .unwrap();
        assert_eq!(
            game.status,
            word_search_backend::models::GameStatus::Finished
        );

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_chat_message() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_chatter").await;

        let game_id = create_solo(&base, &token).await;

        // Connect WS
        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        // Skip initial messages
        let _starting = next_text(&mut rx).await;
        let _joined = next_text(&mut rx).await;

        // Send chat
        let msg = json!({ "type": "chat", "message": "hello world" }).to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        // Receive chat_message broadcast
        let chat = next_text(&mut rx).await;
        assert_eq!(chat["type"], "chat_message");
        assert_eq!(chat["message"], "hello world");

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_ready_message() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_ready").await;

        let game_id = create_solo(&base, &token).await;

        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        let _starting = next_text(&mut rx).await;
        let _joined = next_text(&mut rx).await;

        // Send ready
        let msg = json!({ "type": "ready" }).to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        let ready = next_text(&mut rx).await;
        assert_eq!(ready["type"], "player_ready");

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_invalid_message_format() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_badmsg").await;

        let game_id = create_solo(&base, &token).await;

        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        let _starting = next_text(&mut rx).await;
        let _joined = next_text(&mut rx).await;

        // Send invalid JSON
        let msg = "not valid json at all".to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        let err = next_text(&mut rx).await;
        assert_eq!(err["type"], "error");
        assert!(err["message"]
            .as_str()
            .unwrap()
            .contains("Invalid message format"));

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_duplicate_word_detection() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_duplicate").await;

        let game_id = create_solo(&base, &token).await;

        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        let _starting = next_text(&mut rx).await;
        let _joined = next_text(&mut rx).await;

        // Get first word cells
        let rooms_guard = rooms.read().await;
        let game = rooms_guard
            .values()
            .find(|g| g.id.to_string() == game_id)
            .unwrap();
        let grid = game.grid.as_ref().unwrap();
        let pw = &grid.placed_words[0];
        let cells = cells_for_word(pw);
        drop(rooms_guard);

        // Find the word first time
        let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
        tx.send(Message::Text(msg.clone())).await.unwrap();
        let _found = next_text(&mut rx).await; // word_found
        let _score = next_text(&mut rx).await; // player_score_update

        // Try to find it again
        tx.send(Message::Text(msg)).await.unwrap();
        let err = next_text(&mut rx).await;
        assert_eq!(err["type"], "error");
        assert!(err["message"]
            .as_str()
            .unwrap()
            .contains("already found"));

        tx.close().await.ok();
        shutdown.send(()).ok();
    }
}

// ===========================================================================
// -- Auth module unit tests --------------------------------------------------
// ===========================================================================

mod auth_unit {
    use word_search_backend::auth;

    #[test]
    fn hash_and_verify_password() {
        let hash = auth::hash_password("hunter2").unwrap();
        assert!(auth::verify_password("hunter2", &hash));
        assert!(!auth::verify_password("wrong", &hash));
    }

    #[test]
    fn create_and_verify_jwt() {
        let token = auth::create_jwt("user-123", "alice", "secret").unwrap();
        let claims = auth::verify_jwt(&token, "secret").unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, "alice");
    }

    #[test]
    fn verify_jwt_wrong_secret_fails() {
        let token = auth::create_jwt("user-123", "alice", "secret").unwrap();
        assert!(auth::verify_jwt(&token, "wrong").is_err());
    }
}
