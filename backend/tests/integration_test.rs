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

fn auth_header(token: &str) -> String {
    format!("Bearer {}", token)
}

/// Better helper: register using a shared pool, return token.
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

// ── Auth tests ──────────────────────────────────────────────────────────

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

// ── Game tests ──────────────────────────────────────────────────────────

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
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["id"].is_string());
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
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
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

// ── Discovery tests ────────────────────────────────────────────────────

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
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["service"], "word-search");
}

// ── Auth module unit tests ─────────────────────────────────────────────

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
