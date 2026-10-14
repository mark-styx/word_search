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
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
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

async fn register_with_recovery_pin(pool: &DbPool, username: &str, recovery_pin: &str) -> String {
    let app = build_app(pool.clone());
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "username": username,
                "password": "test123456",
                "recovery_pin": recovery_pin
            })
            .to_string(),
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
async fn sqlite_foreign_keys_are_enabled_for_pooled_connections() {
    let pool = test_pool().await;

    let pragma_value: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pragma_value.0, 1);

    let err = repository::insert_participant(
        &pool,
        &uuid::Uuid::new_v4(),
        &uuid::Uuid::new_v4().to_string(),
        &uuid::Uuid::new_v4().to_string(),
    )
    .await
    .unwrap_err();

    let message = err.to_string().to_ascii_lowercase();
    assert!(
        message.contains("foreign key"),
        "expected foreign key failure, got: {message}"
    );
}

#[tokio::test]
async fn query_indexes_exist_for_history_and_leaderboard_paths() {
    let pool = test_pool().await;

    let participant_indexes: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_index_list('game_participants')")
            .fetch_all(&pool)
            .await
            .unwrap();
    let game_indexes: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_index_list('games')")
        .fetch_all(&pool)
        .await
        .unwrap();
    let user_stats_indexes: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_index_list('user_stats')")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert!(
        participant_indexes
            .iter()
            .any(|index| index.0 == "idx_game_participants_user_id_game_id")
    );
    assert!(
        game_indexes
            .iter()
            .any(|index| index.0 == "idx_games_finished_at_id")
    );
    assert!(
        user_stats_indexes
            .iter()
            .any(|index| index.0 == "idx_user_stats_total_score_user_id")
    );
}

#[tokio::test]
async fn sqlite_pool_uses_wal_and_busy_timeout() {
    let db_path = std::env::temp_dir().join(format!("wordsearch-wal-{}.db", uuid::Uuid::new_v4()));
    let pool = repository::create_pool(db_path.to_str().unwrap())
        .await
        .unwrap();
    repository::run_migrations(&pool).await.unwrap();

    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap();
    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(journal_mode.to_ascii_uppercase(), "WAL");
    assert_eq!(busy_timeout, 5000);

    drop(pool);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn builtin_word_bank_tables_enforce_foreign_keys_and_cascade() {
    let pool = test_pool().await;

    let generated_word_fks: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT \"from\", \"table\", on_delete
         FROM pragma_foreign_key_list('builtin_word_bank_generated_words')",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let usage_fks: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT \"from\", \"table\", on_delete
         FROM pragma_foreign_key_list('builtin_word_bank_word_usage')",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(generated_word_fks.iter().any(|(from, table, on_delete)| {
        from == "theme_key" && table == "builtin_word_bank_state" && on_delete == "CASCADE"
    }));
    assert!(usage_fks.iter().any(|(from, table, on_delete)| {
        from == "theme_key" && table == "builtin_word_bank_state" && on_delete == "CASCADE"
    }));

    let orphan_generated_err = sqlx::query(
        "INSERT INTO builtin_word_bank_generated_words (theme_key, word)
         VALUES ('ghost-theme', 'GHOSTWORD')",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(
        orphan_generated_err
            .to_string()
            .to_ascii_lowercase()
            .contains("foreign key")
    );

    let orphan_usage_err = sqlx::query(
        "INSERT INTO builtin_word_bank_word_usage (theme_key, word, times_used)
         VALUES ('ghost-theme', 'GHOSTWORD', 1)",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(
        orphan_usage_err
            .to_string()
            .to_ascii_lowercase()
            .contains("foreign key")
    );

    repository::ensure_builtin_word_bank_state(&pool, "animals")
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO builtin_word_bank_generated_words (theme_key, word)
         VALUES ('animals', 'GOSLING')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO builtin_word_bank_word_usage (theme_key, word, times_used)
         VALUES ('animals', 'FOX', 2)",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("DELETE FROM builtin_word_bank_state WHERE theme_key = 'animals'")
        .execute(&pool)
        .await
        .unwrap();

    let generated_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM builtin_word_bank_generated_words WHERE theme_key = 'animals'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let usage_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM builtin_word_bank_word_usage WHERE theme_key = 'animals'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(generated_count.0, 0);
    assert_eq!(usage_count.0, 0);
}

#[tokio::test]
async fn word_bank_usage_recording_batches_duplicate_words_without_losing_counts() {
    let pool = test_pool().await;
    let _ = register(&pool, "usageowner").await;
    let owner = repository::get_user_by_username(&pool, "usageowner")
        .await
        .unwrap()
        .unwrap();
    let custom_bank_id = uuid::Uuid::new_v4();

    repository::create_custom_word_bank(&pool, &custom_bank_id, &owner.id, "snacks")
        .await
        .unwrap();

    repository::record_custom_word_bank_usage(
        &pool,
        &custom_bank_id.to_string(),
        &["CHIPS".into(), "CHIPS".into(), "COOKIE".into()],
    )
    .await
    .unwrap();
    repository::record_custom_word_bank_usage(
        &pool,
        &custom_bank_id.to_string(),
        &["CHIPS".into(), "PRETZEL".into()],
    )
    .await
    .unwrap();

    let mut custom_usage = repository::list_custom_word_bank_word_usage(&pool, &custom_bank_id.to_string())
        .await
        .unwrap();
    custom_usage.sort_by(|a, b| a.word.cmp(&b.word));

    assert_eq!(custom_usage.len(), 3);
    assert_eq!(custom_usage[0].word, "CHIPS");
    assert_eq!(custom_usage[0].times_used, 3);
    assert_eq!(custom_usage[1].word, "COOKIE");
    assert_eq!(custom_usage[1].times_used, 1);
    assert_eq!(custom_usage[2].word, "PRETZEL");
    assert_eq!(custom_usage[2].times_used, 1);

    repository::record_builtin_word_bank_usage(
        &pool,
        "animals",
        &["FOX".into(), "FOX".into(), "BEAR".into()],
    )
    .await
    .unwrap();
    repository::record_builtin_word_bank_usage(&pool, "animals", &["FOX".into(), "OWL".into()])
        .await
        .unwrap();

    let mut builtin_usage = repository::list_builtin_word_bank_word_usage(&pool, "animals")
        .await
        .unwrap();
    builtin_usage.sort_by(|a, b| a.word.cmp(&b.word));

    assert_eq!(builtin_usage.len(), 3);
    assert_eq!(builtin_usage[0].word, "BEAR");
    assert_eq!(builtin_usage[0].times_used, 1);
    assert_eq!(builtin_usage[1].word, "FOX");
    assert_eq!(builtin_usage[1].times_used, 3);
    assert_eq!(builtin_usage[2].word, "OWL");
    assert_eq!(builtin_usage[2].times_used, 1);
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
async fn register_invalid_recovery_pin() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "username": "pinny",
                "password": "test123456",
                "recovery_pin": "12ab"
            })
            .to_string(),
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
async fn recover_success() {
    let pool = test_pool().await;
    register_with_recovery_pin(&pool, "recover_me", "2468").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/recover")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "recover_me", "recovery_pin": "2468" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    assert!(v["token"].is_string());
    assert_eq!(v["user"]["username"], "recover_me");
    assert_eq!(v["user"]["has_recovery_pin"], true);
}

#[tokio::test]
async fn recover_wrong_pin() {
    let pool = test_pool().await;
    register_with_recovery_pin(&pool, "recover_wrong", "1357").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/recover")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "recover_wrong", "recovery_pin": "1111" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn recover_requires_enabled_pin() {
    let pool = test_pool().await;
    register(&pool, "no_pin").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/recover")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "username": "no_pin", "recovery_pin": "2468" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_users_returns_prefix_matches() {
    let pool = test_pool().await;
    register_with_recovery_pin(&pool, "alpha_friend", "1234").await;
    register_with_recovery_pin(&pool, "alpha_runner", "5678").await;
    register_with_recovery_pin(&pool, "bravo_pal", "2468").await;

    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/auth/recovery/search?q=alp")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    let usernames = v["usernames"].as_array().unwrap();
    assert_eq!(usernames.len(), 2);
    assert_eq!(usernames[0], "alpha_friend");
    assert_eq!(usernames[1], "alpha_runner");
}

#[tokio::test]
async fn list_players_returns_recent_players() {
    let pool = test_pool().await;
    register(&pool, "player_one").await;
    register(&pool, "player_two").await;

    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/auth/players")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    let players = v["players"].as_array().unwrap();
    assert_eq!(players.len(), 2);
    assert!(players
        .iter()
        .any(|player| player["username"] == "player_one"));
    assert!(players
        .iter()
        .any(|player| player["username"] == "player_two"));
}

#[tokio::test]
async fn create_player_without_password_returns_token() {
    let pool = test_pool().await;
    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/players")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "username": "tap_play" }).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let v = body_json(resp).await;
    assert!(v["token"].is_string());
    assert_eq!(v["user"]["username"], "tap_play");
}

#[tokio::test]
async fn create_guest_returns_token_and_stays_out_of_public_player_list() {
    let pool = test_pool().await;
    let app = build_app(pool.clone());

    let guest_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/guest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(guest_resp.status(), StatusCode::CREATED);

    let guest_json = body_json(guest_resp).await;
    assert!(guest_json["token"].is_string());
    assert_eq!(guest_json["user"]["is_guest"], true);
    let guest_username = guest_json["user"]["username"].as_str().unwrap();
    assert!(guest_username.starts_with("Guest-"));

    let players_resp = app
        .oneshot(Request::builder().uri("/api/auth/players").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(players_resp.status(), StatusCode::OK);

    let players_json = body_json(players_resp).await;
    let players = players_json["players"].as_array().unwrap();
    assert!(players.is_empty());
}

#[tokio::test]
async fn pick_player_by_username_returns_token() {
    let pool = test_pool().await;
    register(&pool, "pick_me").await;

    let app = build_app(pool);
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/players/pick")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "username": "pick_me" }).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    assert!(v["token"].is_string());
    assert_eq!(v["user"]["username"], "pick_me");
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
    assert_eq!(v["highlight_color"], "#3b82f6");
}

#[tokio::test]
async fn update_preferences_persists_and_exposes_player_highlight_color() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);
    let token = register(&pool, "palette_sync").await;

    let req = Request::builder()
        .method("PATCH")
        .uri("/api/auth/me/preferences")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&token))
        .body(Body::from(
            json!({
                "visual_theme": "berry",
                "preferred_word_theme": "cars",
                "highlight_color": "#ff8800"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let prefs = body_json(resp).await;
    assert_eq!(prefs["visual_theme"], "berry");
    assert_eq!(prefs["preferred_word_theme"], "cars");
    assert_eq!(prefs["highlight_color"], "#ff8800");

    let req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&token))
        .body(Body::from(
            json!({ "theme": "cars", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let game_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    let req = Request::builder()
        .uri(&format!("/api/games/{}", game_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = body_json(resp).await;
    assert_eq!(detail["players"][0]["highlight_color"], "#ff8800");
}

#[tokio::test]
async fn update_username_reissues_token_and_updates_active_rooms() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);
    let token = register(&pool, "rename_me").await;

    let create_game_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/games")
                .header("content-type", "application/json")
                .header("authorization", auth_header(&token))
                .body(Body::from(
                    json!({ "theme": "science", "difficulty": "medium" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_game_resp.status(), StatusCode::CREATED);
    let game_id = body_json(create_game_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let rename_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/auth/me/username")
                .header("content-type", "application/json")
                .header("authorization", auth_header(&token))
                .body(Body::from(json!({ "username": "renamed_player" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rename_resp.status(), StatusCode::OK);
    let rename_json = body_json(rename_resp).await;
    let renamed_token = rename_json["token"].as_str().unwrap();
    assert_eq!(rename_json["user"]["username"], "renamed_player");
    assert_ne!(renamed_token, token);

    let me_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("authorization", auth_header(renamed_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_resp.status(), StatusCode::OK);
    assert_eq!(body_json(me_resp).await["username"], "renamed_player");

    let players_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/auth/players").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let players_json = body_json(players_resp).await;
    let players = players_json["players"].as_array().unwrap();
    assert!(players
        .iter()
        .any(|player| player["username"] == "renamed_player"));
    assert!(!players.iter().any(|player| player["username"] == "rename_me"));

    let old_pick_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/players/pick")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "username": "rename_me" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_pick_resp.status(), StatusCode::NOT_FOUND);

    let game_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/games/{game_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(game_resp.status(), StatusCode::OK);
    let game_json = body_json(game_resp).await;
    assert_eq!(game_json["players"][0]["username"], "renamed_player");
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
        "animals",
        "cats",
        "boating",
        "camping",
        "recipe_ingredients",
        "sports",
        "food",
        "fishing",
        "pizza",
        "snacks",
        "fast_food",
        "bugs",
        "movies",
        "cars",
        "desert",
        "mountains",
        "ocean",
        "weather",
        "garden",
        "farm",
        "shiny_things",
        "toys",
        "space",
        "science",
        "geography",
        "technology",
        "music",
        "history",
        "home_renovation",
        "luxury_brands",
        "nature",
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
    let game_id = body_json(resp).await["id"].as_str().unwrap().to_string();

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
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Start game failed: {:?}",
        body_json(resp).await
    );

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
    let game_id = body_json(resp).await["id"].as_str().unwrap().to_string();

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
        .body(Body::from(
            json!({ "theme": "animals", "difficulty": "easy" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"].as_str().unwrap().to_string();

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
        .body(Body::from(
            json!({ "theme": "food", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/start", game_id))
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "First start failed: {:?}",
        body_json(resp).await
    );

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
        .body(Body::from(
            json!({ "theme": "nature", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let game_id = body_json(resp).await["id"].as_str().unwrap().to_string();

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

#[tokio::test]
async fn builtin_word_bank_usage_is_reported_after_solo_game() {
    let pool = test_pool().await;
    let token = register(&pool, "science_tracker").await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms);

    let req = Request::builder()
        .uri("/api/word-banks")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let banks = body_json(resp).await;
    assert_eq!(status, StatusCode::OK, "Actual: {:?}", banks);
    let science_before = banks
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["key"] == "science")
        .unwrap();
    assert_eq!(science_before["coverage_percent"], 0);
    assert_eq!(science_before["total_usage_count"], 0);
    assert_eq!(science_before["can_extend"], true);
    assert_eq!(science_before["extension_state"], "idle");

    let mut launched = false;
    for _ in 0..5 {
        let req = Request::builder()
            .uri("/api/games/solo")
            .header("authorization", auth_header(&token))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        if resp.status() == StatusCode::OK {
            launched = true;
            break;
        }
    }
    assert!(launched, "solo game failed after 5 retries");

    let req = Request::builder()
        .uri("/api/word-banks")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let banks = body_json(resp).await;
    let science_after = banks
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["key"] == "science")
        .unwrap();

    assert!(science_after["used_words_count"].as_u64().unwrap() > 0);
    assert!(science_after["total_usage_count"].as_u64().unwrap() > 0);
    assert!(science_after["coverage_percent"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn private_custom_word_bank_is_hidden_and_cannot_be_used_by_other_players() {
    let pool = test_pool().await;
    let owner_token = register(&pool, "bank_owner").await;
    let viewer_token = register(&pool, "bank_viewer").await;
    let owner = repository::get_user_by_username(&pool, "bank_owner")
        .await
        .unwrap()
        .unwrap();
    let app = build_app(pool.clone());

    let bank_id = uuid::Uuid::new_v4();
    repository::create_custom_word_bank(&pool, &bank_id, &owner.id, "ocean animals")
        .await
        .unwrap();
    repository::update_custom_word_bank_ready(
        &pool,
        &bank_id.to_string(),
        "Ocean Pals",
        &[
            "OCEAN".into(),
            "WAVES".into(),
            "DOLPHIN".into(),
            "TURTLE".into(),
            "STARFISH".into(),
            "SEAHORSE".into(),
            "CORAL".into(),
            "REEF".into(),
            "ANCHOR".into(),
            "TIDAL".into(),
            "SHELL".into(),
            "HARBOR".into(),
            "ISLAND".into(),
            "COAST".into(),
            "MARINE".into(),
            "SAILOR".into(),
            "BEACH".into(),
            "OCTOPUS".into(),
            "WHALE".into(),
            "SHARK".into(),
            "LAGOON".into(),
            "KELP".into(),
            "CURRENT".into(),
            "PEBBLE".into(),
            "TROPICS".into(),
            "SEABIRD".into(),
            "SURF".into(),
            "DRIFTWOOD".into(),
            "MOORING".into(),
            "BOATHOUSE".into(),
            "SEASIDE".into(),
            "LIGHTHOUSE".into(),
            "HORIZON".into(),
            "TIDEPOOL".into(),
            "BREEZE".into(),
            "CATAMARAN".into(),
            "MARINA".into(),
            "PELICAN".into(),
            "DUNE".into(),
            "PADDLE".into(),
            "SEAGRASS".into(),
            "SANDCASTLE".into(),
            "PIER".into(),
            "SEAGLASS".into(),
            "BOARDWALK".into(),
            "SEASTAR".into(),
            "WETSUIT".into(),
            "LIFEGUARD".into(),
        ],
        false,
    )
    .await
    .unwrap();

    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/word-banks")
                .header("authorization", auth_header(&viewer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json = body_json(list_resp).await;
    assert!(
        !list_json
            .as_array()
            .unwrap()
            .iter()
            .any(|bank| bank["id"] == bank_id.to_string())
    );

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/games")
                .header("content-type", "application/json")
                .header("authorization", auth_header(&viewer_token))
                .body(Body::from(
                    json!({ "word_bank_id": bank_id.to_string(), "difficulty": "easy" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::FORBIDDEN);

    let owner_list_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/word-banks")
                .header("authorization", auth_header(&owner_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(owner_list_resp.status(), StatusCode::OK);
    let owner_list_json = body_json(owner_list_resp).await;
    let owner_bank = owner_list_json
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["id"] == bank_id.to_string())
        .unwrap();
    assert_eq!(owner_bank["is_owner"], true);
    assert_eq!(owner_bank["is_shared"], false);
}

#[tokio::test]
async fn owner_can_share_and_delete_custom_word_bank() {
    let pool = test_pool().await;
    let owner_token = register(&pool, "sharing_owner").await;
    let viewer_token = register(&pool, "sharing_viewer").await;
    let owner = repository::get_user_by_username(&pool, "sharing_owner")
        .await
        .unwrap()
        .unwrap();
    let app = build_app(pool.clone());

    let bank_id = uuid::Uuid::new_v4();
    repository::create_custom_word_bank(&pool, &bank_id, &owner.id, "backyard camping")
        .await
        .unwrap();
    repository::update_custom_word_bank_ready(
        &pool,
        &bank_id.to_string(),
        "Backyard Camping",
        &vec![
            "TENT".into(),
            "CAMPFIRE".into(),
            "FLASHLIGHT".into(),
            "SLEEPINGBAG".into(),
            "MARSHMALLOW".into(),
            "THERMOS".into(),
            "LANTERN".into(),
            "BACKPACK".into(),
            "PICNICTABLE".into(),
            "STARGAZING".into(),
            "HAMMOCK".into(),
            "BUGSPRAY".into(),
            "COOLER".into(),
            "KINDLING".into(),
            "COMPASS".into(),
            "BINOCULARS".into(),
            "HEADLAMP".into(),
            "FIREWOOD".into(),
            "SANDWICH".into(),
            "SKILLET".into(),
            "CANOPY".into(),
            "TRAILMIX".into(),
            "WHISTLE".into(),
            "DAYPACK".into(),
            "STOOL".into(),
            "BLANKET".into(),
            "THERMAL".into(),
            "PILLOW".into(),
            "TELESCOPE".into(),
            "SMORES".into(),
            "KETTLE".into(),
            "MUG".into(),
            "MAP".into(),
            "PONCHO".into(),
            "JACKET".into(),
            "GLOVE".into(),
            "MOSS".into(),
            "PINECONE".into(),
            "LOGBOOK".into(),
            "CANTEEN".into(),
            "CAMPSTOOL".into(),
            "MATCHES".into(),
            "CHARCOAL".into(),
            "SPARK".into(),
            "FIREPIT".into(),
            "COYOTE".into(),
            "MOONLIGHT".into(),
            "TWILIGHT".into(),
        ],
        false,
    )
    .await
    .unwrap();

    let share_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/word-banks/{}", bank_id))
                .header("content-type", "application/json")
                .header("authorization", auth_header(&owner_token))
                .body(Body::from(json!({ "is_shared": true }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(share_resp.status(), StatusCode::NO_CONTENT);

    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/word-banks")
                .header("authorization", auth_header(&viewer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json = body_json(list_resp).await;
    let shared_bank = list_json
        .as_array()
        .unwrap()
        .iter()
        .find(|bank| bank["id"] == bank_id.to_string())
        .unwrap();
    assert_eq!(shared_bank["is_shared"], true);
    assert_eq!(shared_bank["owner_username"], "sharing_owner");

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/games")
                .header("content-type", "application/json")
                .header("authorization", auth_header(&viewer_token))
                .body(Body::from(
                    json!({ "word_bank_id": bank_id.to_string(), "difficulty": "easy" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/word-banks/{}", bank_id))
                .header("authorization", auth_header(&owner_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let owner_list_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/word-banks")
                .header("authorization", auth_header(&owner_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(owner_list_resp.status(), StatusCode::OK);
    let owner_list_json = body_json(owner_list_resp).await;
    assert!(
        !owner_list_json
            .as_array()
            .unwrap()
            .iter()
            .any(|bank| bank["id"] == bank_id.to_string())
    );
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
            .body(Body::from(
                json!({ "theme": "animals", "difficulty": "medium" }).to_string(),
            ))
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
    let items = list.as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        assert_eq!(item["theme"], "animals");
        assert_eq!(item["difficulty"], "medium");
    }
}

#[tokio::test]
async fn list_games_rehydrates_lobbies_from_persisted_snapshots() {
    let pool = test_pool().await;
    let rooms: GameRooms =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let app = build_app_shared(pool.clone(), rooms.clone());

    let token = register(&pool, "snapshot_host").await;

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header("authorization", auth_header(&token))
        .body(Body::from(
            json!({ "theme": "animals", "difficulty": "medium" }).to_string(),
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    rooms.write().await.clear();

    let req = Request::builder()
        .uri("/api/games")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = body_json(resp).await;
    let items = list.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["host"], "snapshot_host");

    let rooms_guard = rooms.read().await;
    assert_eq!(rooms_guard.len(), 1);
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
        .header("host", "127.0.0.1:3099")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v["server_name"].is_string());
    assert!(v["hostname"].is_string());
    assert!(v["port"].is_number());
    assert!(v["api_url"].is_string());
    assert!(v["frontend_url"].is_string());
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
    for index in 0..7 {
        let username = format!("leaderboard_{index}");
        register(&pool, &username).await;
        let user = repository::get_user_by_username(&pool, &username)
            .await
            .unwrap()
            .unwrap();

        sqlx::query(
            "INSERT INTO user_stats (user_id, total_games, games_won, total_score, best_single_game_score, total_words_found, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
        )
        .bind(user.id)
        .bind(3_i64)
        .bind(1_i64)
        .bind(100_i64 - index as i64)
        .bind(50_i64)
        .bind(12_i64)
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/leaderboard?page=2&limit=5")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["entries"].as_array().unwrap().len(), 2);
    assert_eq!(v["total"], 7);
}

#[tokio::test]
async fn my_history_reports_true_total_across_pages() {
    let pool = test_pool().await;
    let token = register(&pool, "history_total_user").await;
    let user = repository::get_user_by_username(&pool, "history_total_user")
        .await
        .unwrap()
        .unwrap();

    for index in 0..6 {
        let game_id = uuid::Uuid::new_v4();
        repository::insert_game(
            &pool,
            &game_id,
            "science",
            "medium",
            "finished",
            None,
            false,
            4,
        )
        .await
        .unwrap();
        repository::update_game_status(&pool, &game_id.to_string(), "finished")
            .await
            .unwrap();
        repository::insert_participant(
            &pool,
            &uuid::Uuid::new_v4(),
            &game_id.to_string(),
            &user.id,
        )
        .await
        .unwrap();
        repository::update_participant_stats(
            &pool,
            &game_id.to_string(),
            &user.id,
            10 + index,
            2 + index,
            Some(1),
        )
        .await
        .unwrap();
    }

    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/me/history?page=2&limit=2")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["entries"].as_array().unwrap().len(), 2);
    assert_eq!(v["total"], 6);
}

#[tokio::test]
async fn found_words_are_unique_per_game_word() {
    let pool = test_pool().await;
    register(&pool, "claim_first").await;
    register(&pool, "claim_second").await;

    let first_user = repository::get_user_by_username(&pool, "claim_first")
        .await
        .unwrap()
        .unwrap();
    let second_user = repository::get_user_by_username(&pool, "claim_second")
        .await
        .unwrap()
        .unwrap();

    let game_id = uuid::Uuid::new_v4();
    repository::insert_game(&pool, &game_id, "animals", "easy", "in_progress", None, false, 4)
        .await
        .unwrap();

    let first_result = repository::claim_found_word(
        &pool,
        &uuid::Uuid::new_v4(),
        &game_id.to_string(),
        &first_user.id,
        "LION",
        "2026-01-01T00:00:01Z",
    )
    .await
    .unwrap();
    let second_result = repository::claim_found_word(
        &pool,
        &uuid::Uuid::new_v4(),
        &game_id.to_string(),
        &second_user.id,
        "LION",
        "2026-01-01T00:00:02Z",
    )
    .await
    .unwrap();

    assert!(matches!(
        first_result,
        repository::ClaimFoundWordResult::Won { previous_claim: None }
    ));
    assert!(matches!(
        second_result,
        repository::ClaimFoundWordResult::Lost { .. }
    ));

    let claim = repository::get_found_word_claim(&pool, &game_id.to_string(), "LION")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.user_id, first_user.id);

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM found_words WHERE game_id = ? AND word = ?",
    )
    .bind(game_id.to_string())
    .bind("LION")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn earlier_claim_timestamp_replaces_later_claim_even_if_written_second() {
    let pool = test_pool().await;
    register(&pool, "claim_earlier").await;
    register(&pool, "claim_later").await;

    let earlier_user = repository::get_user_by_username(&pool, "claim_earlier")
        .await
        .unwrap()
        .unwrap();
    let later_user = repository::get_user_by_username(&pool, "claim_later")
        .await
        .unwrap()
        .unwrap();

    let game_id = uuid::Uuid::new_v4();
    repository::insert_game(&pool, &game_id, "animals", "easy", "in_progress", None, false, 4)
        .await
        .unwrap();

    let first_result = repository::claim_found_word(
        &pool,
        &uuid::Uuid::new_v4(),
        &game_id.to_string(),
        &later_user.id,
        "OTTER",
        "2026-01-01T00:00:02Z",
    )
    .await
    .unwrap();
    let second_result = repository::claim_found_word(
        &pool,
        &uuid::Uuid::new_v4(),
        &game_id.to_string(),
        &earlier_user.id,
        "OTTER",
        "2026-01-01T00:00:01Z",
    )
    .await
    .unwrap();

    assert!(matches!(
        first_result,
        repository::ClaimFoundWordResult::Won { previous_claim: None }
    ));
    match second_result {
        repository::ClaimFoundWordResult::Won {
            previous_claim: Some(previous_claim),
        } => {
            assert_eq!(previous_claim.user_id, later_user.id);
            assert_eq!(previous_claim.found_at, "2026-01-01T00:00:02Z");
        }
        other => panic!("expected earlier claimant to replace later claim, got {other:?}"),
    }

    let claim = repository::get_found_word_claim(&pool, &game_id.to_string(), "OTTER")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.user_id, earlier_user.id);
    assert_eq!(claim.found_at, "2026-01-01T00:00:01Z");
}

#[tokio::test]
async fn my_stats_reports_found_word_timing_metrics() {
    let pool = test_pool().await;
    let token = register(&pool, "timed_stats_user").await;
    let user = repository::get_user_by_username(&pool, "timed_stats_user")
        .await
        .unwrap()
        .unwrap();

    let timing_cases: [&[i64]; 2] = [&[1, 3], &[5]];
    for (index, offsets) in timing_cases.into_iter().enumerate() {
        let game_id = uuid::Uuid::new_v4();
        repository::insert_game(
            &pool,
            &game_id,
            "science",
            "medium",
            "finished",
            None,
            false,
            4,
        )
        .await
        .unwrap();

        let started_at = if index == 0 {
            "2026-01-01T00:00:00Z"
        } else {
            "2026-01-01T01:00:00Z"
        };
        let finished_at = if index == 0 {
            "2026-01-01T00:05:00Z"
        } else {
            "2026-01-01T01:05:00Z"
        };
        sqlx::query("UPDATE games SET started_at = ?, finished_at = ?, status = 'finished' WHERE id = ?")
            .bind(started_at)
            .bind(finished_at)
            .bind(game_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        repository::insert_participant(
            &pool,
            &uuid::Uuid::new_v4(),
            &game_id.to_string(),
            &user.id,
        )
        .await
        .unwrap();
        repository::update_participant_stats(
            &pool,
            &game_id.to_string(),
            &user.id,
            20,
            offsets.len() as i64,
            Some(1),
        )
        .await
        .unwrap();

        for (word_index, offset_seconds) in offsets.iter().enumerate() {
            let found_at = if index == 0 {
                format!("2026-01-01T00:00:0{}Z", offset_seconds)
            } else {
                format!("2026-01-01T01:00:0{}Z", offset_seconds)
            };
            repository::insert_found_word(
                &pool,
                &uuid::Uuid::new_v4(),
                &game_id.to_string(),
                &user.id,
                &format!("WORD{}", word_index + index * 10),
                &found_at,
            )
            .await
            .unwrap();
        }
    }

    repository::upsert_user_stats(&pool, &user.id).await.unwrap();

    let app = build_app(pool);
    let req = Request::builder()
        .uri("/api/stats/me")
        .header("authorization", auth_header(&token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["fastest_word_find_ms"], 1000);
    assert_eq!(v["average_word_find_ms"], 3000);
}

// ===========================================================================
// -- WebSocket integration tests ---------------------------------------------
// ===========================================================================

mod ws_tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    /// Spin up a real HTTP server on a random port, return (base_url, rooms, pool, shutdown_tx).
    async fn start_server() -> (String, GameRooms, DbPool, tokio::sync::oneshot::Sender<()>) {
        let pool = test_pool().await;
        let rooms: GameRooms =
            std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
        let app = build_app_shared(pool.clone(), rooms.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
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
        let (stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        stream.split()
    }

    /// Read the next text message from a WebSocket stream.
    async fn next_text_with_timeout(
        rx: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        timeout: std::time::Duration,
    ) -> serde_json::Value {
        loop {
            let msg = tokio::time::timeout(timeout, rx.next())
                .await
                .expect("Timeout waiting for WS message")
                .expect("WS stream ended")
                .expect("WS error");
            match msg {
                Message::Text(text) => return serde_json::from_str(&text).unwrap(),
                Message::Ping(_) | Message::Pong(_) => continue,
                other => panic!("Expected text message, got: {:?}", other),
            }
        }
    }

    async fn next_text(
        rx: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> serde_json::Value {
        next_text_with_timeout(rx, std::time::Duration::from_secs(5)).await
    }

    async fn next_text_of_type(
        rx: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        message_type: &str,
    ) -> serde_json::Value {
        for _ in 0..10 {
            let message = next_text(rx).await;
            if message["type"] == message_type {
                return message;
            }
        }

        panic!("Did not receive WS message type '{message_type}' after 10 attempts");
    }

    async fn next_text_of_type_with_timeout(
        rx: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        message_type: &str,
        timeout: std::time::Duration,
    ) -> serde_json::Value {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let now = tokio::time::Instant::now();
            assert!(
                now < deadline,
                "Did not receive WS message type '{message_type}' before timeout",
            );
            let remaining = deadline.saturating_duration_since(now);
            let message = next_text_with_timeout(rx, remaining).await;
            if message["type"] == message_type {
                return message;
            }
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
                return resp.json::<serde_json::Value>().await.unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_string();
            }
        }
        panic!("Solo game creation failed after 5 retries");
    }

    /// Get cells JSON for a placed word.
    fn cells_for_word(pw: &word_search_backend::models::PlacedWord) -> Vec<serde_json::Value> {
        let (dr, dc) = pw.direction.delta();
        (0..pw.word.len())
            .map(|i| {
                let r = (pw.start.0 as isize + dr * i as isize) as usize;
                let c = (pw.start.1 as isize + dc * i as isize) as usize;
                json!([r, c])
            })
            .collect()
    }

    async fn room_snapshot(
        rooms: &GameRooms,
        game_id: &str,
    ) -> word_search_backend::models::GameState {
        let game_uuid = uuid::Uuid::parse_str(game_id).unwrap();
        let room = {
            let rooms_guard = rooms.read().await;
            rooms_guard.get(&game_uuid).cloned().unwrap()
        };
        let game = room.read().await;
        game.clone()
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

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_connect_also_receives_authoritative_game_state_sync() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_sync_host").await;

        let game_id = create_solo(&base, &token).await;

        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        let starting = next_text(&mut rx).await;
        assert_eq!(starting["type"], "game_starting");

        let sync = next_text_of_type(&mut rx, "game_state_sync").await;
        assert_eq!(sync["game"]["id"], game_id);
        assert_eq!(sync["game"]["status"], "in_progress");
        assert!(sync["game"]["grid"].is_object());

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_invalid_token_rejected() {
        let (base, _rooms, _pool, shutdown) = start_server().await;

        let ws_url = base.replace("http://", "ws://");
        let url = format!(
            "{}/ws?token=invalid&game_id=00000000-0000-0000-0000-000000000000",
            ws_url
        );

        let result = tokio_tungstenite::connect_async(&url).await;
        // The server should reject the connection
        assert!(result.is_err(), "Connection with invalid token should fail");

        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_start_game_notifies_connected_players() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "ws_start_host").await;
        let player_token = register(&pool, "ws_start_player").await;
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy" }))
            .send()
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let join_resp = client
            .post(format!("{}/api/games/{}/join", base, game_id))
            .header("authorization", auth_header(&player_token))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);

        let (mut host_tx, mut host_rx) = connect_ws(&base, &host_token, &game_id).await;
        let (mut player_tx, mut player_rx) = connect_ws(&base, &player_token, &game_id).await;

        let start_resp = client
            .post(format!("{}/api/games/{}/start", base, game_id))
            .header("authorization", auth_header(&host_token))
            .send()
            .await
            .unwrap();
        assert_eq!(start_resp.status(), StatusCode::OK);

        let host_msg = next_text_of_type(&mut host_rx, "game_starting").await;
        let player_msg = next_text_of_type(&mut player_rx, "game_starting").await;
        assert!(host_msg["grid"].is_object());
        assert!(player_msg["grid"].is_object());

        host_tx.close().await.ok();
        player_tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_select_cells_and_find_word() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_finder").await;

        let game_id = create_solo(&base, &token).await;

        // Connect WS
        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        // Skip initial game_starting
        let _starting = next_text(&mut rx).await;
        let _sync = next_text_of_type(&mut rx, "game_state_sync").await;

        // Get grid info to find a valid word selection
        let game = room_snapshot(&rooms, &game_id).await;
        let grid = game.grid.as_ref().unwrap();
        let first_word = &grid.placed_words[0];
        let cells = cells_for_word(first_word);
        let word_text = first_word.word.clone();

        // Send select_cells
        let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        // Should receive word_found
        let found = next_text_of_type(&mut rx, "word_found").await;
        assert_eq!(found["type"], "word_found");
        assert_eq!(found["word"], word_text);

        // Should receive player_score_update
        let score_update = next_text_of_type(&mut rx, "player_score_update").await;
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

        // Skip initial message
        let _starting = next_text(&mut rx).await;
        let _sync = next_text_of_type(&mut rx, "game_state_sync").await;

        // Get all placed words
        let game = room_snapshot(&rooms, &game_id).await;
        let grid = game.grid.as_ref().unwrap();
        let words_info: Vec<_> = grid
            .placed_words
            .iter()
            .map(|pw| (pw.word.clone(), cells_for_word(pw)))
            .collect();

        // Find each word
        for (i, (_word, cells)) in words_info.iter().enumerate() {
            let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
            tx.send(Message::Text(msg)).await.unwrap();

            // word_found
            let found = next_text_of_type(&mut rx, "word_found").await;
            assert_eq!(
                found["type"], "word_found",
                "Expected word_found for word #{}, got: {:?}",
                i, found
            );

            // player_score_update
            let score = next_text_of_type(&mut rx, "player_score_update").await;
            assert_eq!(score["type"], "player_score_update");

            // If this was the last word, we also get game_finished
            if i == words_info.len() - 1 {
                let finished = next_text_of_type(&mut rx, "game_finished").await;
                assert_eq!(finished["type"], "game_finished");
                let rankings = finished["rankings"].as_array().unwrap();
                assert_eq!(rankings.len(), 1);
                assert!(rankings[0]["score"].as_u64().unwrap() > 0);
            }
        }

        // Verify game state is finished
        let game = room_snapshot(&rooms, &game_id).await;
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

        // Skip initial message
        let _starting = next_text(&mut rx).await;
        let _sync = next_text_of_type(&mut rx, "game_state_sync").await;

        // Send chat
        let msg = json!({ "type": "chat", "message": "hello world" }).to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        // Receive chat_message broadcast
        let chat = next_text_of_type(&mut rx, "chat_message").await;
        assert_eq!(chat["type"], "chat_message");
        assert_eq!(chat["message"], "hello world");

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_ready_message() {
        let (base, _rooms, pool, shutdown) = start_server().await;
        let token = register(&pool, "ws_ready").await;
        let client = reqwest::Client::new();

        let game_id = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&token))
            .json(&json!({
                "theme": "animals",
                "difficulty": "easy",
                "max_players": 4
            }))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let (mut tx, mut rx) = connect_ws(&base, &token, &game_id).await;

        let _sync = next_text_of_type(&mut rx, "game_state_sync").await;

        // Send ready
        let msg = json!({ "type": "ready" }).to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        let ready = next_text_of_type(&mut rx, "player_ready").await;
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
        let _sync = next_text_of_type(&mut rx, "game_state_sync").await;

        // Send invalid JSON
        let msg = "not valid json at all".to_string();
        tx.send(Message::Text(msg)).await.unwrap();

        let err = next_text_of_type(&mut rx, "error").await;
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
        let _sync = next_text_of_type(&mut rx, "game_state_sync").await;

        // Get first word cells
        let game = room_snapshot(&rooms, &game_id).await;
        let grid = game.grid.as_ref().unwrap();
        let pw = &grid.placed_words[0];
        let cells = cells_for_word(pw);

        // Find the word first time
        let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
        tx.send(Message::Text(msg.clone())).await.unwrap();
        let _found = next_text_of_type(&mut rx, "word_found").await;
        let _score = next_text_of_type(&mut rx, "player_score_update").await;

        // Try to find it again
        tx.send(Message::Text(msg)).await.unwrap();
        let err = next_text_of_type(&mut rx, "error").await;
        assert_eq!(err["type"], "error");
        assert!(err["message"].as_str().unwrap().contains("already found"));

        tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_earlier_claim_reconciles_stale_room_winner() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "ws_claim_host").await;
        let challenger_token = register(&pool, "ws_claim_challenger").await;
        let host_user = repository::get_user_by_username(&pool, "ws_claim_host")
            .await
            .unwrap()
            .unwrap();
        let challenger_user = repository::get_user_by_username(&pool, "ws_claim_challenger")
            .await
            .unwrap()
            .unwrap();
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy" }))
            .send()
            .await
            .unwrap();
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let join_resp = client
            .post(format!("{}/api/games/{}/join", base, game_id))
            .header("authorization", auth_header(&challenger_token))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);

        let start_resp = client
            .post(format!("{}/api/games/{}/start", base, game_id))
            .header("authorization", auth_header(&host_token))
            .send()
            .await
            .unwrap();
        assert_eq!(start_resp.status(), StatusCode::OK);

        let snapshot = room_snapshot(&rooms, &game_id).await;
        let grid = snapshot.grid.as_ref().unwrap().clone();
        let first_word = grid.placed_words[0].clone();
        let cells = cells_for_word(&first_word);
        let word = first_word.word.clone();
        let score = snapshot.difficulty.score_for_word(&word);
        let future_claimed_at = chrono::Utc::now() + chrono::Duration::seconds(5);

        let game_uuid = uuid::Uuid::parse_str(&game_id).unwrap();
        let room = api::get_room_handle(&rooms, game_uuid).await.unwrap();
        {
            let mut game = room.write().await;
            if let Some(player) = game.players.get_mut(&host_user.id.parse().unwrap()) {
                player.score = score;
            }
            game.found_words
                .insert(host_user.id.parse().unwrap(), vec![word.clone()]);
            game.word_claims.insert(
                word.clone(),
                word_search_backend::models::WordClaim {
                    player_id: host_user.id.parse().unwrap(),
                    claimed_at: future_claimed_at,
                },
            );
            game.touch();
        }
        repository::restore_found_word_claim(
            &pool,
            &uuid::Uuid::new_v4(),
            &game_id,
            &host_user.id,
            &word,
            &future_claimed_at.to_rfc3339(),
        )
        .await
        .unwrap();

        let (mut challenger_tx, mut challenger_rx) =
            connect_ws(&base, &challenger_token, &game_id).await;
        let _starting = next_text_of_type(&mut challenger_rx, "game_starting").await;
        let _sync = next_text_of_type(&mut challenger_rx, "game_state_sync").await;

        let msg = json!({ "type": "select_cells", "cells": cells }).to_string();
        challenger_tx.send(Message::Text(msg)).await.unwrap();

        let found = next_text_of_type(&mut challenger_rx, "word_found").await;
        assert_eq!(found["word"], word);

        let sync = next_text_of_type(&mut challenger_rx, "game_state_sync").await;
        let players = sync["game"]["players"].as_array().unwrap();
        let host_entry = players
            .iter()
            .find(|player| player["id"] == host_user.id)
            .unwrap();
        let challenger_entry = players
            .iter()
            .find(|player| player["id"] == challenger_user.id)
            .unwrap();
        assert_eq!(host_entry["score"], 0);
        assert_eq!(challenger_entry["score"], score);

        let room_state = room_snapshot(&rooms, &game_id).await;
        assert!(
            !room_state
                .found_words
                .get(&host_user.id.parse().unwrap())
                .is_some_and(|words| words.iter().any(|found_word| found_word == &word))
        );
        assert!(
            room_state
                .found_words
                .get(&challenger_user.id.parse().unwrap())
                .is_some_and(|words| words.iter().any(|found_word| found_word == &word))
        );
        assert_eq!(
            room_state.word_claims.get(&word).map(|claim| claim.player_id),
            Some(challenger_user.id.parse().unwrap())
        );

        let persisted_claim = repository::get_found_word_claim(&pool, &game_id, &word)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted_claim.user_id, challenger_user.id);

        challenger_tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_leave_game_removes_lobby_player_and_reassigns_host() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "leave_host").await;
        let player_token = register(&pool, "leave_player").await;
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy" }))
            .send()
            .await
            .unwrap();
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let join_resp = client
            .post(format!("{}/api/games/{}/join", base, game_id))
            .header("authorization", auth_header(&player_token))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);

        let (mut host_tx, mut host_rx) = connect_ws(&base, &host_token, &game_id).await;
        let (mut player_tx, mut player_rx) = connect_ws(&base, &player_token, &game_id).await;

        let _host_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let _player_sync = next_text_of_type(&mut player_rx, "game_state_sync").await;

        player_tx
            .send(Message::Text(json!({ "type": "leave_game" }).to_string()))
            .await
            .unwrap();

        let left = next_text_of_type(&mut host_rx, "player_left").await;
        assert_eq!(left["player_id"].as_str().unwrap().len(), 36);
        let sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let players = sync["game"]["players"].as_array().unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0]["username"], "leave_host");
        assert_eq!(players[0]["is_host"], true);

        let game = room_snapshot(&rooms, &game_id).await;
        assert_eq!(game.players.len(), 1);
        assert!(game.players.values().next().unwrap().is_host);

        host_tx.close().await.ok();
        player_tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_disconnect_marks_in_progress_player_disconnected_after_grace_period() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "disconnect_host").await;
        let player_token = register(&pool, "disconnect_player").await;
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy" }))
            .send()
            .await
            .unwrap();
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let join_resp = client
            .post(format!("{}/api/games/{}/join", base, game_id))
            .header("authorization", auth_header(&player_token))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);

        let (mut host_tx, mut host_rx) = connect_ws(&base, &host_token, &game_id).await;
        let (mut player_tx, mut player_rx) = connect_ws(&base, &player_token, &game_id).await;

        let _host_lobby_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let _player_lobby_sync = next_text_of_type(&mut player_rx, "game_state_sync").await;

        let start_resp = client
            .post(format!("{}/api/games/{}/start", base, game_id))
            .header("authorization", auth_header(&host_token))
            .send()
            .await
            .unwrap();
        assert_eq!(start_resp.status(), StatusCode::OK);

        let _host_started = next_text_of_type(&mut host_rx, "game_starting").await;
        let _host_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let _player_started = next_text_of_type(&mut player_rx, "game_starting").await;
        let _player_sync = next_text_of_type(&mut player_rx, "game_state_sync").await;

        player_tx.close().await.ok();
        tokio::time::sleep(std::time::Duration::from_secs(13)).await;

        let game = room_snapshot(&rooms, &game_id).await;
        let player = game
            .players
            .values()
            .find(|player| player.username == "disconnect_player")
            .unwrap();
        assert!(!player.connected);
        assert_eq!(game.players.len(), 2);

        host_tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_time_limited_game_finishes_when_timer_expires() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "timeout_host").await;
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy", "time_limit_secs": 1 }))
            .send()
            .await
            .unwrap();
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let (mut host_tx, mut host_rx) = connect_ws(&base, &host_token, &game_id).await;
        let _lobby_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;

        let start_resp = client
            .post(format!("{}/api/games/{}/start", base, game_id))
            .header("authorization", auth_header(&host_token))
            .send()
            .await
            .unwrap();
        assert_eq!(start_resp.status(), StatusCode::OK);

        let _started = next_text_of_type(&mut host_rx, "game_starting").await;
        let _in_progress_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let finished = next_text_of_type_with_timeout(
            &mut host_rx,
            "game_finished",
            std::time::Duration::from_secs(4),
        )
        .await;
        assert_eq!(finished["reason"], "timeout");

        let sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        assert_eq!(sync["game"]["status"], "finished");
        assert_eq!(sync["game"]["finished_reason"], "timeout");

        let game = room_snapshot(&rooms, &game_id).await;
        assert_eq!(game.status, word_search_backend::models::GameStatus::Finished);
        assert_eq!(
            game.finished_reason,
            Some(word_search_backend::models::GameFinishReason::Timeout)
        );

        host_tx.close().await.ok();
        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_game_is_abandoned_when_all_in_progress_players_disconnect() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "abandon_host").await;
        let player_token = register(&pool, "abandon_player").await;
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy" }))
            .send()
            .await
            .unwrap();
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let join_resp = client
            .post(format!("{}/api/games/{}/join", base, game_id))
            .header("authorization", auth_header(&player_token))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);

        let (mut host_tx, mut host_rx) = connect_ws(&base, &host_token, &game_id).await;
        let (mut player_tx, mut player_rx) = connect_ws(&base, &player_token, &game_id).await;

        let _host_lobby_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let _player_lobby_sync = next_text_of_type(&mut player_rx, "game_state_sync").await;

        let start_resp = client
            .post(format!("{}/api/games/{}/start", base, game_id))
            .header("authorization", auth_header(&host_token))
            .send()
            .await
            .unwrap();
        assert_eq!(start_resp.status(), StatusCode::OK);

        let _host_started = next_text_of_type(&mut host_rx, "game_starting").await;
        let _host_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let _player_started = next_text_of_type(&mut player_rx, "game_starting").await;
        let _player_sync = next_text_of_type(&mut player_rx, "game_state_sync").await;

        player_tx.close().await.ok();
        host_tx.close().await.ok();

        tokio::time::sleep(std::time::Duration::from_secs(13)).await;

        let game = room_snapshot(&rooms, &game_id).await;
        assert_eq!(game.status, word_search_backend::models::GameStatus::Finished);
        assert_eq!(
            game.finished_reason,
            Some(word_search_backend::models::GameFinishReason::Abandoned)
        );

        shutdown.send(()).ok();
    }

    #[tokio::test]
    async fn ws_heartbeat_timeout_removes_idle_lobby_player() {
        let (base, rooms, pool, shutdown) = start_server().await;
        let host_token = register(&pool, "heartbeat_host").await;
        let player_token = register(&pool, "heartbeat_player").await;
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/api/games", base))
            .header("authorization", auth_header(&host_token))
            .json(&json!({ "theme": "animals", "difficulty": "easy" }))
            .send()
            .await
            .unwrap();
        let game_id = create_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let join_resp = client
            .post(format!("{}/api/games/{}/join", base, game_id))
            .header("authorization", auth_header(&player_token))
            .send()
            .await
            .unwrap();
        assert_eq!(join_resp.status(), StatusCode::OK);

        let (mut host_tx, mut host_rx) = connect_ws(&base, &host_token, &game_id).await;
        let (_player_tx, _player_rx) = connect_ws(&base, &player_token, &game_id).await;

        let _host_sync = next_text_of_type(&mut host_rx, "game_state_sync").await;

        let left = next_text_of_type_with_timeout(
            &mut host_rx,
            "player_left",
            std::time::Duration::from_secs(20),
        )
        .await;
        assert_eq!(left["type"], "player_left");

        let sync = next_text_of_type(&mut host_rx, "game_state_sync").await;
        let players = sync["game"]["players"].as_array().unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0]["username"], "heartbeat_host");

        let game = room_snapshot(&rooms, &game_id).await;
        assert_eq!(game.players.len(), 1);
        assert!(game.players.values().all(|player| player.username != "heartbeat_player"));

        host_tx.close().await.ok();
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
