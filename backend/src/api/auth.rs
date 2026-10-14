use axum::{
    extract::Extension,
    extract::Query,
    http::StatusCode,
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{get_room_handle, GameChannels, GameRooms};
use crate::auth::{self, AuthUser};
use crate::db::repository::{self, DbPool};
use crate::ws::room;

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    recovery_pin: Option<String>,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RecoverRequest {
    username: String,
    recovery_pin: String,
}

#[derive(Deserialize)]
struct PlayerAuthRequest {
    username: String,
}

#[derive(Deserialize)]
struct SearchUsersRequest {
    q: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    user: UserResponse,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    created_at: Option<String>,
    visual_theme: String,
    preferred_word_theme: Option<String>,
    highlight_color: String,
    has_recovery_pin: bool,
    is_guest: bool,
}

#[derive(Serialize)]
struct UsernameSearchResponse {
    usernames: Vec<String>,
}

#[derive(Serialize)]
struct PublicPlayerResponse {
    id: String,
    username: String,
    created_at: String,
}

#[derive(Serialize)]
struct PlayerListResponse {
    players: Vec<PublicPlayerResponse>,
}

#[derive(Deserialize)]
struct UpdatePreferencesRequest {
    visual_theme: String,
    preferred_word_theme: Option<String>,
    highlight_color: String,
}

#[derive(Deserialize)]
struct UpdateUsernameRequest {
    username: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/recover", post(recover))
        .route("/guest", post(create_guest))
        .route("/players", get(list_players).post(create_player))
        .route("/players/pick", post(pick_player))
        .route("/recovery/search", get(search_users))
}

pub fn protected_routes() -> Router {
    Router::new()
        .route("/me", get(me))
        .route("/me/username", patch(update_username))
        .route("/me/preferences", patch(update_preferences))
}

async fn register(
    Extension(pool): Extension<DbPool>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    // Validate input
    if body.username.len() < 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must be at least 3 characters".into(),
        ));
    }
    if body.password.len() < 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 6 characters".into(),
        ));
    }
    if let Some(pin) = body.recovery_pin.as_deref() {
        if !is_valid_recovery_pin(pin) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Recovery PIN must be 4 digits".into(),
            ));
        }
    }

    // Check username uniqueness
    if repository::get_user_by_username(&pool, &body.username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "Username already taken".into()));
    }

    let user_id = Uuid::new_v4();
    let hash = auth::hash_password(&body.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let recovery_pin_hash = match body.recovery_pin.as_deref() {
        Some(pin) => Some(
            auth::hash_password(pin)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        ),
        None => None,
    };

    repository::create_user(
        &pool,
        &user_id,
        &body.username,
        &hash,
        recovery_pin_hash.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let secret = auth::jwt_secret();
    let token = auth::create_jwt(&user_id.to_string(), &body.username, &secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = repository::get_user_by_id(&pool, &user_id.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User was not created".into(),
        ))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: user_row_to_response(&user),
        }),
    ))
}

async fn login(
    Extension(pool): Extension<DbPool>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let user = repository::get_user_by_username(&pool, &body.username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials".into()))?;

    if !auth::verify_password(&body.password, &user.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into()));
    }

    let secret = auth::jwt_secret();
    let token = auth::create_jwt(&user.id, &user.username, &secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: user_row_to_response(&user),
    }))
}

async fn recover(
    Extension(pool): Extension<DbPool>,
    Json(body): Json<RecoverRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    if !is_valid_recovery_pin(&body.recovery_pin) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Recovery PIN must be 4 digits".into(),
        ));
    }

    let user = repository::get_user_by_username(&pool, &body.username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid recovery details".into()))?;

    let Some(recovery_pin_hash) = user.recovery_pin_hash.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            "This player does not have recovery enabled".into(),
        ));
    };

    if !auth::verify_password(&body.recovery_pin, recovery_pin_hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid recovery details".into()));
    }

    let secret = auth::jwt_secret();
    let token = auth::create_jwt(&user.id, &user.username, &secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: user_row_to_response(&user),
    }))
}

async fn list_players(
    Extension(pool): Extension<DbPool>,
    Query(query): Query<SearchUsersRequest>,
) -> Result<Json<PlayerListResponse>, (StatusCode, String)> {
    let players =
        repository::list_public_users(&pool, query.q.as_deref(), query.limit.unwrap_or(60))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PlayerListResponse {
        players: players
            .into_iter()
            .map(|player| PublicPlayerResponse {
                id: player.id,
                username: player.username,
                created_at: player.created_at,
            })
            .collect(),
    }))
}

async fn create_player(
    Extension(pool): Extension<DbPool>,
    Json(body): Json<PlayerAuthRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    let username = body.username.trim();
    if username.len() < 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must be at least 3 characters".into(),
        ));
    }

    if repository::get_user_by_username(&pool, username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "Username already taken".into()));
    }

    let user_id = Uuid::new_v4();
    let generated_secret = format!("player-{}", Uuid::new_v4().simple());
    let hash = auth::hash_password(&generated_secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    repository::create_user(&pool, &user_id, username, &hash, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = repository::get_user_by_id(&pool, &user_id.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "User was not created".into(),
        ))?;

    Ok((StatusCode::CREATED, Json(issue_auth_response(&user)?)))
}

async fn create_guest(
    Extension(pool): Extension<DbPool>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    let user_id = Uuid::new_v4();
    let username = next_guest_username(&pool).await?;
    let generated_secret = format!("guest-{}", Uuid::new_v4().simple());
    let hash = auth::hash_password(&generated_secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    repository::create_guest_user(&pool, &user_id, &username, &hash)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = repository::get_user_by_id(&pool, &user_id.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Guest user was not created".into(),
        ))?;

    Ok((StatusCode::CREATED, Json(issue_auth_response(&user)?)))
}

async fn pick_player(
    Extension(pool): Extension<DbPool>,
    Json(body): Json<PlayerAuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let username = body.username.trim();
    if username.len() < 3 {
        return Err((StatusCode::BAD_REQUEST, "Choose a valid player".into()));
    }

    let user = repository::get_user_by_username(&pool, username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Player not found".into()))?;
    if user.is_guest != 0 {
        return Err((StatusCode::NOT_FOUND, "Player not found".into()));
    }

    Ok(Json(issue_auth_response(&user)?))
}

async fn search_users(
    Extension(pool): Extension<DbPool>,
    Query(query): Query<SearchUsersRequest>,
) -> Result<Json<UsernameSearchResponse>, (StatusCode, String)> {
    let Some(raw_query) = query.q.as_deref() else {
        return Ok(Json(UsernameSearchResponse {
            usernames: Vec::new(),
        }));
    };
    let term = raw_query.trim();
    if term.len() < 2 {
        return Ok(Json(UsernameSearchResponse {
            usernames: Vec::new(),
        }));
    }

    let usernames = repository::search_usernames(&pool, term, 8)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UsernameSearchResponse { usernames }))
}

async fn me(
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    Ok(Json(user_row_to_response(&user)))
}

async fn update_preferences(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<UpdatePreferencesRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    if body.visual_theme.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Visual theme is required".into()));
    }
    if !is_valid_highlight_color(&body.highlight_color) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Highlight color must be a hex color".into(),
        ));
    }

    repository::update_user_preferences(
        &pool,
        &auth_user.user_id,
        body.visual_theme.trim(),
        body.preferred_word_theme.as_deref(),
        &body.highlight_color.to_lowercase(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let room_ids: Vec<Uuid> = rooms.read().await.keys().copied().collect();
    for game_id in room_ids {
        let Some(room) = get_room_handle(&rooms, game_id).await else {
            continue;
        };
        let mut game = room.write().await;
        for player in game.players.values_mut() {
            if player.id.to_string() == auth_user.user_id {
                player.highlight_color = body.highlight_color.to_lowercase();
            }
        }
    }

    let user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    Ok(Json(user_row_to_response(&user)))
}

async fn update_username(
    Extension(pool): Extension<DbPool>,
    Extension(rooms): Extension<GameRooms>,
    Extension(channels): Extension<GameChannels>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<UpdateUsernameRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let next_username = body.username.trim();
    if next_username.len() < 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must be at least 3 characters".into(),
        ));
    }

    let current_user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    if current_user.username == next_username {
        return Ok(Json(issue_auth_response(&current_user)?));
    }

    if let Some(existing_user) = repository::get_user_by_username(&pool, next_username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        if existing_user.id != auth_user.user_id {
            return Err((StatusCode::CONFLICT, "Username already taken".into()));
        }
    }

    repository::update_user_username(&pool, &auth_user.user_id, next_username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated_user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    let room_ids: Vec<Uuid> = rooms.read().await.keys().copied().collect();
    for game_id in room_ids {
        let Some(room) = get_room_handle(&rooms, game_id).await else {
            continue;
        };

        let snapshot = {
            let mut game = room.write().await;
            let mut changed = false;

            for player in game.players.values_mut() {
                if player.id.to_string() == auth_user.user_id && player.username != updated_user.username
                {
                    player.username = updated_user.username.clone();
                    changed = true;
                }
            }

            if changed {
                game.touch();
                Some(game.clone())
            } else {
                None
            }
        };

        let Some(snapshot) = snapshot else {
            continue;
        };

        room::persist_game_state_snapshot(&pool, &snapshot).await;
        if let Some(tx) = channels.read().await.get(&game_id).cloned() {
            let _ = tx.send(room::game_state_sync_message(&snapshot));
        }
    }

    Ok(Json(issue_auth_response(&updated_user)?))
}

fn user_row_to_response(user: &repository::UserRow) -> UserResponse {
    UserResponse {
        id: user.id.clone(),
        username: user.username.clone(),
        created_at: Some(user.created_at.clone()),
        visual_theme: user.visual_theme.clone(),
        preferred_word_theme: user.preferred_word_theme.clone(),
        highlight_color: user.highlight_color.clone(),
        has_recovery_pin: user.recovery_pin_hash.is_some(),
        is_guest: user.is_guest != 0,
    }
}

fn issue_auth_response(user: &repository::UserRow) -> Result<AuthResponse, (StatusCode, String)> {
    let secret = auth::jwt_secret();
    let token = auth::create_jwt(&user.id, &user.username, &secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(AuthResponse {
        token,
        user: user_row_to_response(user),
    })
}

fn is_valid_highlight_color(value: &str) -> bool {
    let hex = value.strip_prefix('#').unwrap_or(value);
    matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_recovery_pin(value: &str) -> bool {
    value.len() == 4 && value.chars().all(|c| c.is_ascii_digit())
}

async fn next_guest_username(pool: &DbPool) -> Result<String, (StatusCode, String)> {
    for _ in 0..32 {
        let raw = Uuid::new_v4().simple().to_string();
        let candidate = format!("Guest-{}", raw[..6].to_ascii_uppercase());
        let exists = repository::get_user_by_username(pool, &candidate)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .is_some();
        if !exists {
            return Ok(candidate);
        }
    }

    Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Could not allocate a guest name".into(),
    ))
}
