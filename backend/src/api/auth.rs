use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{self, AuthUser};
use crate::db::repository::{self, DbPool};

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
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
}

pub fn routes() -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
}

async fn register(
    Extension(pool): Extension<DbPool>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    // Validate input
    if body.username.len() < 3 {
        return Err((StatusCode::BAD_REQUEST, "Username must be at least 3 characters".into()));
    }
    if body.password.len() < 6 {
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 6 characters".into()));
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

    repository::create_user(&pool, &user_id, &body.username, &hash)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me".into());
    let token = auth::create_jwt(&user_id.to_string(), &body.username, &secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id: user_id.to_string(),
                username: body.username,
            },
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

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me".into());
    let token = auth::create_jwt(&user.id, &user.username, &secret)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            username: user.username,
        },
    }))
}

async fn me(
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let user = repository::get_user_by_id(&pool, &auth_user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
    }))
}
