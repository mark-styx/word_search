use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub username: String,
    pub exp: usize, // unix timestamp
}

static JWT_SECRET: OnceLock<String> = OnceLock::new();
const INSECURE_JWT_SECRETS: &[&str] = &["change-me", "change-me-in-production"];

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt =
        argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn create_jwt(
    user_id: &str,
    username: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn jwt_secret() -> &'static str {
    JWT_SECRET
        .get_or_init(|| {
            let (secret, ephemeral) = resolve_jwt_secret(std::env::var("JWT_SECRET").ok());
            if ephemeral {
                tracing::warn!(
                    "JWT_SECRET is unset or uses an insecure placeholder; using an ephemeral process-local secret. Tokens will be invalidated on restart."
                );
            }
            secret
        })
        .as_str()
}

fn resolve_jwt_secret(candidate: Option<String>) -> (String, bool) {
    match candidate {
        Some(secret) if !is_insecure_jwt_secret(&secret) => (secret, false),
        _ => (generated_jwt_secret(), true),
    }
}

fn is_insecure_jwt_secret(secret: &str) -> bool {
    let normalized = secret.trim();
    normalized.is_empty() || INSECURE_JWT_SECRETS.contains(&normalized)
}

fn generated_jwt_secret() -> String {
    format!(
        "ephemeral-{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

/// Extension to hold authenticated user info on the request.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
}

/// Axum middleware that extracts and validates JWT from Authorization header.
/// Sets `AuthUser` as a request extension on success.
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let secret = jwt_secret();

    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    match verify_jwt(token, &secret) {
        Ok(claims) => {
            request.extensions_mut().insert(AuthUser {
                user_id: claims.sub,
                username: claims.username,
            });
            Ok(next.run(request).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_jwt_secret;

    #[test]
    fn preserves_explicit_secret() {
        let (secret, ephemeral) = resolve_jwt_secret(Some("top-secret".into()));
        assert_eq!(secret, "top-secret");
        assert!(!ephemeral);
    }

    #[test]
    fn replaces_known_placeholder_secret() {
        let (secret, ephemeral) = resolve_jwt_secret(Some("change-me".into()));
        assert_ne!(secret, "change-me");
        assert!(ephemeral);
    }

    #[test]
    fn replaces_blank_secret() {
        let (secret, ephemeral) = resolve_jwt_secret(Some("   ".into()));
        assert!(!secret.trim().is_empty());
        assert!(ephemeral);
    }
}
