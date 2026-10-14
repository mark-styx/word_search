use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::repository::{self, DbPool};
use crate::puzzles::catalog;

#[derive(Serialize)]
struct WordBankSummary {
    id: String,
    key: String,
    label: String,
    source: String,
    status: String,
    total_words: usize,
    used_words_count: usize,
    total_usage_count: usize,
    coverage_percent: u8,
    can_extend: bool,
    extension_state: String,
    error_message: Option<String>,
    last_extension_error: Option<String>,
    created_at: Option<String>,
    owner_username: Option<String>,
    is_owner: bool,
    is_shared: bool,
}

#[derive(Deserialize)]
struct GenerateWordBankRequest {
    prompt: String,
}

#[derive(Serialize)]
struct GenerateWordBankResponse {
    id: String,
    status: String,
}

#[derive(Deserialize)]
struct UpdateWordBankRequest {
    is_shared: bool,
}

pub fn protected_routes() -> Router {
    Router::new()
        .route("/", get(list_word_banks))
        .route("/generate", post(generate_word_bank))
        .route("/:id", patch(update_word_bank).delete(delete_word_bank))
}

async fn list_word_banks(
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<WordBankSummary>>, (StatusCode, String)> {
    let builtin_state = repository::list_builtin_word_banks(&pool)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let builtin_state_by_key: HashMap<String, repository::BuiltInWordBankRow> = builtin_state
        .into_iter()
        .map(|row| (row.theme_key.clone(), row))
        .collect();

    let mut banks: Vec<WordBankSummary> = catalog::built_in_word_banks()
        .iter()
        .map(|entry| {
            let state = builtin_state_by_key.get(entry.key);
            let total_words = catalog::get_all_words(entry.theme).len()
                + state
                    .map(|row| row.generated_words_count.max(0) as usize)
                    .unwrap_or(0);
            let used_words_count = state
                .map(|row| row.used_words_count.max(0) as usize)
                .unwrap_or(0);
            let total_usage_count = state
                .map(|row| row.total_usage_count.max(0) as usize)
                .unwrap_or(0);
            let coverage_percent = if total_words > 0 {
                ((used_words_count as f64 / total_words as f64) * 100.0).round() as u8
            } else {
                0
            };

            WordBankSummary {
                id: entry.key.to_string(),
                key: entry.key.to_string(),
                label: entry.label.to_string(),
                source: "builtin".into(),
                status: "ready".into(),
                total_words,
                used_words_count,
                total_usage_count,
                coverage_percent,
                can_extend: state.map(|row| row.can_extend != 0).unwrap_or(true),
                extension_state: state
                    .map(|row| row.extension_state.clone())
                    .unwrap_or_else(|| "idle".into()),
                error_message: None,
                last_extension_error: state.and_then(|row| row.last_extension_error.clone()),
                created_at: None,
                owner_username: None,
                is_owner: false,
                is_shared: true,
            }
        })
        .collect();

    let custom = repository::list_visible_custom_word_banks(&pool, &auth_user.user_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    banks.extend(custom.into_iter().map(|entry| {
        let coverage = if entry.total_words > 0 {
            ((entry.used_words_count as f64 / entry.total_words as f64) * 100.0).round() as u8
        } else {
            0
        };

        WordBankSummary {
            id: entry.id.clone(),
            key: format!("custom:{}", entry.id),
            label: entry
                .label
                .unwrap_or_else(|| "Generating Word Bank".to_string()),
            source: "custom".into(),
            status: entry.status,
            total_words: entry.total_words.max(0) as usize,
            used_words_count: entry.used_words_count.max(0) as usize,
            total_usage_count: entry.total_usage_count.max(0) as usize,
            coverage_percent: coverage,
            can_extend: entry.can_extend != 0,
            extension_state: entry.extension_state,
            error_message: entry.error_message,
            last_extension_error: entry.last_extension_error,
            created_at: Some(entry.created_at),
            owner_username: Some(entry.owner_username),
            is_owner: entry.user_id == auth_user.user_id,
            is_shared: entry.is_shared != 0,
        }
    }));

    Ok(Json(banks))
}

async fn generate_word_bank(
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<GenerateWordBankRequest>,
) -> Result<(StatusCode, Json<GenerateWordBankResponse>), (StatusCode, String)> {
    let prompt = body.prompt.trim();
    if prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Please enter a word or phrase".into(),
        ));
    }

    let word_bank_id = Uuid::new_v4();
    repository::create_custom_word_bank(&pool, &word_bank_id, &auth_user.user_id, prompt)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    tokio::spawn(crate::word_banks::generate_and_store_custom_word_bank(
        pool.clone(),
        word_bank_id.to_string(),
        prompt.to_string(),
    ));

    Ok((
        StatusCode::ACCEPTED,
        Json(GenerateWordBankResponse {
            id: word_bank_id.to_string(),
            status: "generating".into(),
        }),
    ))
}

async fn update_word_bank(
    Path(id): Path<String>,
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<UpdateWordBankRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let updated = repository::update_custom_word_bank_sharing(
        &pool,
        &id,
        &auth_user.user_id,
        body.is_shared,
    )
    .await
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    if !updated {
        return Err((StatusCode::NOT_FOUND, "Custom word bank not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_word_bank(
    Path(id): Path<String>,
    Extension(pool): Extension<DbPool>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = repository::delete_custom_word_bank(&pool, &id, &auth_user.user_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, "Custom word bank not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}
