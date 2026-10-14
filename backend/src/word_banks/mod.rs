use chrono::{DateTime, Utc};
use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::{fs, process::Command};

use crate::db::repository::{self, DbPool};
use crate::models::WordTheme;
use crate::puzzles::catalog;

const GENERATED_WORD_COUNT: usize = 72;
const MIN_READY_WORD_COUNT: usize = 48;
const MAX_WORD_LENGTH: usize = 15;
const MAX_NAME_LENGTH: usize = 48;
const REFILL_TRIGGER_COVERAGE: f64 = 0.58;
const MAX_EXTENSION_WORDS: usize = 18;
const MIN_EXTENSION_WORDS: usize = 8;

#[derive(Debug, Deserialize)]
struct GeneratedWordBank {
    name: String,
    can_extend: bool,
    words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GenerationReview {
    approved: bool,
    can_extend: bool,
    reason: String,
    words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionProposal {
    can_extend: bool,
    reason: String,
    words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionReview {
    approved: bool,
    can_extend: bool,
    reason: String,
    words: Vec<String>,
}

pub async fn generate_and_store_custom_word_bank(
    pool: DbPool,
    word_bank_id: String,
    user_prompt: String,
) {
    let result = run_generation(&user_prompt).await;

    match result {
        Ok(generated) => {
            if let Err(error) = repository::update_custom_word_bank_ready(
                &pool,
                &word_bank_id,
                &generated.name,
                &generated.words,
                generated.can_extend,
            )
            .await
            {
                tracing::error!(
                    word_bank_id = %word_bank_id,
                    error = %error,
                    "failed to persist generated word bank"
                );
            }
        }
        Err(error) => {
            tracing::warn!(
                word_bank_id = %word_bank_id,
                error = %error,
                "custom word bank generation failed"
            );

            let _ = repository::update_custom_word_bank_failed(&pool, &word_bank_id, &error).await;
        }
    }
}

pub async fn maybe_schedule_extension(pool: DbPool, word_bank_id: String) {
    let row = match repository::get_custom_word_bank_by_id(&pool, &word_bank_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(word_bank_id = %word_bank_id, error = %error, "failed to inspect word bank extension state");
            return;
        }
    };

    if row.status != "ready"
        || row.can_extend == 0
        || row.extension_state == "extending"
        || row.total_words <= 0
    {
        return;
    }

    let coverage = row.used_words_count as f64 / row.total_words as f64;
    if coverage < REFILL_TRIGGER_COVERAGE {
        return;
    }

    if let Err(error) = repository::update_custom_word_bank_extension_state(
        &pool,
        &word_bank_id,
        "extending",
        true,
        None,
    )
    .await
    {
        tracing::warn!(word_bank_id = %word_bank_id, error = %error, "failed to mark extension as active");
        return;
    }

    tokio::spawn(async move {
        extend_and_store_custom_word_bank(pool, word_bank_id).await;
    });
}

pub async fn maybe_schedule_builtin_extension(pool: DbPool, theme: WordTheme) {
    let theme_key = catalog::theme_key(theme).to_string();
    if let Err(error) = repository::ensure_builtin_word_bank_state(&pool, &theme_key).await {
        tracing::warn!(theme_key = %theme_key, error = %error, "failed to initialize built-in word bank state");
        return;
    }

    let row = match repository::get_builtin_word_bank_by_theme(&pool, &theme_key).await {
        Ok(Some(row)) => row,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(theme_key = %theme_key, error = %error, "failed to inspect built-in word bank extension state");
            return;
        }
    };

    if row.can_extend == 0 || row.extension_state == "extending" {
        return;
    }

    let total_words = catalog::get_all_words(theme).len() as i64 + row.generated_words_count.max(0);
    if total_words <= 0 {
        return;
    }

    let coverage = row.used_words_count as f64 / total_words as f64;
    if coverage < REFILL_TRIGGER_COVERAGE {
        return;
    }

    if let Err(error) = repository::update_builtin_word_bank_extension_state(
        &pool,
        &theme_key,
        "extending",
        true,
        None,
    )
    .await
    {
        tracing::warn!(theme_key = %theme_key, error = %error, "failed to mark built-in extension as active");
        return;
    }

    tokio::spawn(async move {
        extend_and_store_builtin_word_bank(pool, theme).await;
    });
}

pub fn custom_word_weight(times_used: i64, last_used_at: Option<&str>, now: DateTime<Utc>) -> f64 {
    let usage_penalty = 1.0 / (1.0 + times_used as f64 * 1.8);
    let recency_penalty = last_used_at
        .and_then(parse_timestamp)
        .map(|timestamp| {
            let hours = now.signed_duration_since(timestamp).num_hours().max(0) as f64;
            if hours < 6.0 {
                0.06
            } else if hours < 24.0 {
                0.15
            } else if hours < 72.0 {
                0.35
            } else {
                1.0
            }
        })
        .unwrap_or(1.0);

    (usage_penalty * recency_penalty).max(0.02)
}

async fn extend_and_store_custom_word_bank(pool: DbPool, word_bank_id: String) {
    let row = match repository::get_custom_word_bank_by_id(&pool, &word_bank_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(word_bank_id = %word_bank_id, error = %error, "failed to load word bank for extension");
            return;
        }
    };

    let label = match row.label {
        Some(label) if !label.trim().is_empty() => label,
        _ => {
            let _ = repository::update_custom_word_bank_extension_state(
                &pool,
                &word_bank_id,
                "failed",
                row.can_extend != 0,
                Some("Word bank is missing a label"),
            )
            .await;
            return;
        }
    };

    let existing_words = match row.words_json {
        Some(raw) => match serde_json::from_str::<Vec<String>>(&raw) {
            Ok(words) => words,
            Err(error) => {
                let _ = repository::update_custom_word_bank_extension_state(
                    &pool,
                    &word_bank_id,
                    "failed",
                    row.can_extend != 0,
                    Some(&format!("Stored words are invalid: {error}")),
                )
                .await;
                return;
            }
        },
        None => {
            let _ = repository::update_custom_word_bank_extension_state(
                &pool,
                &word_bank_id,
                "failed",
                row.can_extend != 0,
                Some("Stored words are missing"),
            )
            .await;
            return;
        }
    };

    match run_custom_extension(&label, &row.prompt, &existing_words).await {
        Ok(review) if review.approved && !review.words.is_empty() => {
            if let Err(error) = repository::append_custom_word_bank_words(
                &pool,
                &word_bank_id,
                &review.words,
                review.can_extend,
            )
            .await
            {
                tracing::warn!(word_bank_id = %word_bank_id, error = %error, "failed to append extension words");
                let _ = repository::update_custom_word_bank_extension_state(
                    &pool,
                    &word_bank_id,
                    "failed",
                    review.can_extend,
                    Some(&error.to_string()),
                )
                .await;
            }
        }
        Ok(review) => {
            let state = if review.can_extend {
                "idle"
            } else {
                "disabled"
            };
            let _ = repository::update_custom_word_bank_extension_state(
                &pool,
                &word_bank_id,
                state,
                review.can_extend,
                Some(&review.reason),
            )
            .await;
        }
        Err(error) => {
            let next_state = if is_transient_generator_unavailable(&error) {
                "idle"
            } else {
                "failed"
            };
            let _ = repository::update_custom_word_bank_extension_state(
                &pool,
                &word_bank_id,
                next_state,
                row.can_extend != 0,
                Some(&error),
            )
            .await;
        }
    }
}

async fn extend_and_store_builtin_word_bank(pool: DbPool, theme: WordTheme) {
    let theme_key = catalog::theme_key(theme).to_string();
    let row = match repository::get_builtin_word_bank_by_theme(&pool, &theme_key).await {
        Ok(Some(row)) => row,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(theme_key = %theme_key, error = %error, "failed to load built-in word bank for extension");
            return;
        }
    };

    let label = catalog::theme_label(theme).to_string();
    let mut existing_words: Vec<String> = catalog::get_all_words(theme)
        .iter()
        .map(|word| (*word).to_string())
        .collect();
    match repository::list_builtin_generated_words(&pool, &theme_key).await {
        Ok(words) => existing_words.extend(words),
        Err(error) => {
            let _ = repository::update_builtin_word_bank_extension_state(
                &pool,
                &theme_key,
                "failed",
                row.can_extend != 0,
                Some(&error.to_string()),
            )
            .await;
            return;
        }
    }

    match run_extension(&label, &label, &existing_words).await {
        Ok(review) if review.approved && !review.words.is_empty() => {
            if let Err(error) = repository::append_builtin_word_bank_words(
                &pool,
                &theme_key,
                &review.words,
                review.can_extend,
            )
            .await
            {
                tracing::warn!(theme_key = %theme_key, error = %error, "failed to append built-in extension words");
                let _ = repository::update_builtin_word_bank_extension_state(
                    &pool,
                    &theme_key,
                    "failed",
                    review.can_extend,
                    Some(&error.to_string()),
                )
                .await;
            }
        }
        Ok(review) => {
            let state = if review.can_extend {
                "idle"
            } else {
                "disabled"
            };
            let _ = repository::update_builtin_word_bank_extension_state(
                &pool,
                &theme_key,
                state,
                review.can_extend,
                Some(&review.reason),
            )
            .await;
        }
        Err(error) => {
            let next_state = if is_transient_generator_unavailable(&error) {
                "idle"
            } else {
                "failed"
            };
            let _ = repository::update_builtin_word_bank_extension_state(
                &pool,
                &theme_key,
                next_state,
                row.can_extend != 0,
                Some(&error),
            )
            .await;
        }
    }
}

async fn run_generation(user_prompt: &str) -> Result<GeneratedWordBank, String> {
    let parsed: GeneratedWordBank = match run_codex_json(
        build_generation_prompt(user_prompt),
        generation_schema_json(),
    )
    .await
    {
        Ok(parsed) => parsed,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "external word bank generator unavailable; refusing unsafe local custom-bank fallback"
            );
            return Err(custom_generation_unavailable_message());
        }
    };

    let generated = normalize_generated_word_bank(parsed)?;
    let review: GenerationReview = run_codex_json(
        build_generation_review_prompt(user_prompt, &generated.name, &generated.words),
        generation_review_schema_json(),
    )
    .await
    .map_err(|error| {
        tracing::warn!(
            error = %error,
            bank_name = generated.name,
            "custom word bank generation review unavailable; refusing unreviewed result"
        );
        custom_generation_unavailable_message()
    })?;

    apply_generation_review(generated, review)
}

async fn run_custom_extension(
    label: &str,
    original_prompt: &str,
    existing_words: &[String],
) -> Result<ExtensionReview, String> {
    let proposal: ExtensionProposal = run_codex_json(
        build_extension_prompt(label, original_prompt, existing_words),
        extension_schema_json(),
    )
    .await
    .map_err(|error| {
        tracing::warn!(
            error = %error,
            label = label,
            "external word bank extension unavailable; refusing unsafe local custom-bank fallback"
        );
        custom_generation_unavailable_message()
    })?;

    let normalized_candidates = normalize_words(&proposal.words, existing_words);
    if normalized_candidates.len() < MIN_EXTENSION_WORDS {
        return Ok(ExtensionReview {
            approved: false,
            can_extend: proposal.can_extend,
            reason: proposal.reason,
            words: Vec::new(),
        });
    }

    let review: ExtensionReview = run_codex_json(
        build_extension_review_prompt(
            label,
            original_prompt,
            existing_words,
            &normalized_candidates,
            &proposal.reason,
        ),
        extension_review_schema_json(),
    )
    .await
    .map_err(|error| {
        tracing::warn!(
            error = %error,
            label = label,
            "external word bank extension review unavailable; refusing unsafe local custom-bank fallback"
        );
        custom_generation_unavailable_message()
    })?;

    let approved_words = normalize_words(&review.words, existing_words);
    Ok(ExtensionReview {
        approved: review.approved && approved_words.len() >= MIN_EXTENSION_WORDS,
        can_extend: review.can_extend,
        reason: review.reason,
        words: approved_words,
    })
}

async fn run_extension(
    label: &str,
    original_prompt: &str,
    existing_words: &[String],
) -> Result<ExtensionReview, String> {
    let proposal: ExtensionProposal = run_codex_json(
        build_extension_prompt(label, original_prompt, existing_words),
        extension_schema_json(),
    )
    .await
    .map_err(|error| {
        tracing::warn!(
            error = %error,
            label = label,
            "external word bank extension unavailable; refusing unsafe built-in fallback"
        );
        custom_generation_unavailable_message()
    })?;
    let normalized_candidates = normalize_words(&proposal.words, existing_words);

    if normalized_candidates.len() < MIN_EXTENSION_WORDS {
        return Ok(ExtensionReview {
            approved: false,
            can_extend: proposal.can_extend,
            reason: proposal.reason,
            words: Vec::new(),
        });
    }

    let review: ExtensionReview = run_codex_json(
        build_extension_review_prompt(
            label,
            original_prompt,
            existing_words,
            &normalized_candidates,
            &proposal.reason,
        ),
        extension_review_schema_json(),
    )
    .await
    .map_err(|error| {
        tracing::warn!(
            error = %error,
            label = label,
            "external word bank extension review unavailable; refusing unsafe built-in fallback"
        );
        custom_generation_unavailable_message()
    })?;

    let approved_words = normalize_words(&review.words, existing_words);
    Ok(ExtensionReview {
        approved: review.approved && approved_words.len() >= MIN_EXTENSION_WORDS,
        can_extend: review.can_extend,
        reason: review.reason,
        words: approved_words,
    })
}

async fn run_codex_json<T>(prompt: String, schema: String) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let temp_dir = std::env::temp_dir();
    let schema_path = temp_dir.join(format!("word-bank-schema-{}.json", uuid::Uuid::new_v4()));
    let output_path = temp_dir.join(format!("word-bank-output-{}.json", uuid::Uuid::new_v4()));

    fs::write(&schema_path, schema)
        .await
        .map_err(|error| format!("Failed to prepare Codex schema: {error}"))?;

    let mut command = Command::new(generator_binary());
    command.arg("exec");
    command.arg("--skip-git-repo-check");
    command.arg("--ephemeral");
    command.arg("--sandbox");
    command.arg("read-only");
    command.arg("--color");
    command.arg("never");
    command.arg("--output-schema");
    command.arg(&schema_path);
    command.arg("--output-last-message");
    command.arg(&output_path);
    command.arg("-C");
    command.arg(workspace_root());

    if let Ok(model) = std::env::var("WORD_BANK_GENERATOR_MODEL") {
        if !model.trim().is_empty() {
            command.arg("--model");
            command.arg(model);
        }
    }

    command.arg(prompt);

    let output = command
        .output()
        .await
        .map_err(|error| format!("Failed to launch Codex CLI: {error}"))?;

    let _ = fs::remove_file(&schema_path).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "Codex CLI exited with a non-zero status".to_string()
        };
        let _ = fs::remove_file(&output_path).await;
        return Err(detail);
    }

    let raw = fs::read_to_string(&output_path)
        .await
        .map_err(|error| format!("Failed to read Codex output: {error}"))?;
    let _ = fs::remove_file(&output_path).await;

    serde_json::from_str(&raw).map_err(|error| format!("Codex returned invalid JSON: {error}"))
}

fn normalize_generated_word_bank(value: GeneratedWordBank) -> Result<GeneratedWordBank, String> {
    let name = normalize_name(&value.name);
    if name.is_empty() {
        return Err("Codex did not provide a valid family-friendly name".into());
    }

    let words = normalize_words(&value.words, &[]);
    if words.len() < MIN_READY_WORD_COUNT {
        return Err(format!(
            "Codex returned only {} usable words; need at least {}",
            words.len(),
            MIN_READY_WORD_COUNT
        ));
    }

    Ok(GeneratedWordBank {
        name,
        can_extend: value.can_extend,
        words,
    })
}

fn apply_generation_review(
    generated: GeneratedWordBank,
    review: GenerationReview,
) -> Result<GeneratedWordBank, String> {
    let approved_words = normalize_words(&review.words, &[]);

    if !review.approved {
        return Err(format!(
            "Generated bank rejected for theme drift: {}",
            review.reason
        ));
    }

    if approved_words.len() < MIN_READY_WORD_COUNT {
        return Err(format!(
            "Generated bank rejected because only {} words survived theme review; need at least {}. {}",
            approved_words.len(),
            MIN_READY_WORD_COUNT,
            review.reason
        ));
    }

    Ok(GeneratedWordBank {
        name: generated.name,
        can_extend: generated.can_extend && review.can_extend,
        words: approved_words,
    })
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == ' ' || *character == '&'
        })
        .take(MAX_NAME_LENGTH)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_words(words: &[String], existing_words: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen: HashSet<String> = existing_words.iter().cloned().collect();

    for word in words {
        let normalized_word = normalize_word(word);
        if normalized_word.len() < 2 || normalized_word.len() > MAX_WORD_LENGTH {
            continue;
        }
        if seen.insert(normalized_word.clone()) {
            normalized.push(normalized_word);
        }
        if normalized.len() >= GENERATED_WORD_COUNT {
            break;
        }
    }

    normalized
}

fn normalize_word(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f %z")
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .ok()
        })
}

fn generator_binary() -> String {
    std::env::var("WORD_BANK_GENERATOR_BIN").unwrap_or_else(|_| "codex".into())
}

fn custom_generation_unavailable_message() -> String {
    "The external word-bank generator is unavailable right now, so this word bank cannot safely create new theme-accurate words. Try again when the generator is available.".to_string()
}

fn is_transient_generator_unavailable(error: &str) -> bool {
    error.contains("theme-accurate words")
        || error.contains("generator is unavailable")
        || error.contains("Failed to launch Codex CLI")
}

pub fn log_generator_status() {
    static LOGGED: OnceLock<()> = OnceLock::new();

    if LOGGED.set(()).is_err() {
        return;
    }

    let binary = generator_binary();
    if generator_binary_exists(&binary) {
        tracing::info!(
            binary = %binary,
            "external word bank generator available"
        );
    } else {
        tracing::warn!(
            binary = %binary,
            "external word bank generator not found; custom generation and safe bank expansion will be unavailable"
        );
    }
}

pub async fn built_in_words(theme: WordTheme, pool: &DbPool) -> Result<Vec<String>, String> {
    let theme_key = catalog::theme_key(theme).to_string();
    let mut words: Vec<String> = catalog::get_all_words(theme)
        .iter()
        .map(|word| (*word).to_string())
        .collect();
    let generated = repository::list_builtin_generated_words(pool, &theme_key)
        .await
        .map_err(|error| error.to_string())?;

    let mut seen = HashSet::new();
    words.extend(generated);
    words.retain(|word| seen.insert(word.clone()));
    Ok(words)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn generator_binary_exists(binary: &str) -> bool {
    if binary.contains(std::path::MAIN_SEPARATOR) || binary.contains('/') || binary.contains('\\') {
        let path = Path::new(binary);
        return path.is_absolute() && path.exists()
            || std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(path).exists())
                .unwrap_or(false);
    }

    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|entry| {
                let candidate = entry.join(binary);
                candidate.exists()
            })
        })
        .unwrap_or(false)
}

fn build_generation_prompt(user_prompt: &str) -> String {
    let prompt = user_prompt.replace('\n', " ").replace('"', "'");
    format!(
        "Create a family-friendly word-search word bank from this user request: \"{prompt}\".\n\
\n\
Rules:\n\
- Always keep the result appropriate for kids, classrooms, and family game night.\n\
- If the request is violent, sexual, hateful, scary, drug-related, or otherwise not family-friendly, ignore it and create a wholesome general-interest family-friendly bank instead.\n\
- Invent a short display name for the frontend in title case.\n\
- Set can_extend to false only if the topic is inherently narrow and likely to run out of good matching words quickly.\n\
- Return exactly one JSON object with keys \"name\", \"can_extend\", and \"words\".\n\
- Provide {GENERATED_WORD_COUNT} unique words.\n\
- Every word must be a single token that can be normalized into only A-Z letters.\n\
- Avoid profanity, adult themes, religion, politics, real-world tragedy, gore, and slurs.\n\
- Favor cheerful, educational, playful, everyday topics.\n\
- Word length should be between 3 and {MAX_WORD_LENGTH} characters before normalization when possible.\n\
- No explanations, markdown, code fences, or extra keys."
    )
}

fn build_generation_review_prompt(
    user_prompt: &str,
    generated_name: &str,
    generated_words: &[String],
) -> String {
    let prompt = user_prompt.replace('\n', " ").replace('"', "'");
    let words = generated_words.join(", ");
    format!(
        "Adversarially review a generated custom word-search bank for theme accuracy.\n\
\n\
User request: \"{prompt}\"\n\
Generated bank name: {generated_name}\n\
Generated words: {words}\n\
\n\
Review rules:\n\
- Theme fidelity is the top priority.\n\
- Reject any word that is clearly off-theme, generic filler, or better suited to a different topic.\n\
- Keep only words that strongly fit the requested theme while remaining family-friendly.\n\
- If the bank as a whole drifts away from the requested topic, set approved to false.\n\
- If fewer than {MIN_READY_WORD_COUNT} words can be kept without theme drift, set approved to false and return only the words that truly fit.\n\
- can_extend should be false if the topic is too narrow to safely keep adding fresh on-theme words.\n\
- Return JSON only with keys \"approved\", \"can_extend\", \"reason\", and \"words\"."
    )
}

fn build_extension_prompt(label: &str, original_prompt: &str, existing_words: &[String]) -> String {
    let prompt = original_prompt.replace('\n', " ").replace('"', "'");
    let existing = existing_words.join(", ");
    format!(
        "You are extending an existing family-friendly word-search bank.\n\
\n\
Bank name: {label}\n\
Original request: \"{prompt}\"\n\
Existing words: {existing}\n\
\n\
Task:\n\
- Decide whether this bank should keep growing or whether the theme is too narrow to extend safely.\n\
- If it can grow, propose up to {MAX_EXTENSION_WORDS} NEW words that clearly fit the theme and remain family-friendly.\n\
- If it should not grow, return no words and set can_extend to false.\n\
- Return JSON only with keys \"can_extend\", \"reason\", and \"words\"."
    )
}

fn build_extension_review_prompt(
    label: &str,
    original_prompt: &str,
    existing_words: &[String],
    candidate_words: &[String],
    proposal_reason: &str,
) -> String {
    let prompt = original_prompt.replace('\n', " ").replace('"', "'");
    let existing = existing_words.join(", ");
    let candidates = candidate_words.join(", ");
    format!(
        "Adversarially review proposed additions for a family-friendly word-search bank.\n\
\n\
Bank name: {label}\n\
Original request: \"{prompt}\"\n\
Existing words: {existing}\n\
Proposal reason: {proposal_reason}\n\
Candidate new words: {candidates}\n\
\n\
Review rules:\n\
- Reject any word that is off-theme, too specific to a different topic, not family-friendly, or redundant with the current bank.\n\
- If the theme is exhausted or too narrow, set approved to false, can_extend to false, and return an empty words array.\n\
- Otherwise return only approved words.\n\
- Return JSON only with keys \"approved\", \"can_extend\", \"reason\", and \"words\"."
    )
}

fn generation_schema_json() -> String {
    format!(
        r#"{{
  "type": "object",
  "additionalProperties": false,
  "required": ["name", "can_extend", "words"],
  "properties": {{
    "name": {{
      "type": "string",
      "minLength": 1,
      "maxLength": {MAX_NAME_LENGTH}
    }},
    "can_extend": {{
      "type": "boolean"
    }},
    "words": {{
      "type": "array",
      "minItems": {MIN_READY_WORD_COUNT},
      "maxItems": 96,
      "items": {{
        "type": "string",
        "minLength": 2,
        "maxLength": {MAX_WORD_LENGTH}
      }}
    }}
  }}
}}"#
    )
}

fn generation_review_schema_json() -> String {
    format!(
        r#"{{
  "type": "object",
  "additionalProperties": false,
  "required": ["approved", "can_extend", "reason", "words"],
  "properties": {{
    "approved": {{ "type": "boolean" }},
    "can_extend": {{ "type": "boolean" }},
    "reason": {{ "type": "string", "minLength": 1, "maxLength": 300 }},
    "words": {{
      "type": "array",
      "minItems": 0,
      "maxItems": 96,
      "items": {{
        "type": "string",
        "minLength": 2,
        "maxLength": {MAX_WORD_LENGTH}
      }}
    }}
  }}
}}"#
    )
}

fn extension_schema_json() -> String {
    format!(
        r#"{{
  "type": "object",
  "additionalProperties": false,
  "required": ["can_extend", "reason", "words"],
  "properties": {{
    "can_extend": {{ "type": "boolean" }},
    "reason": {{ "type": "string", "minLength": 1, "maxLength": 300 }},
    "words": {{
      "type": "array",
      "maxItems": {MAX_EXTENSION_WORDS},
      "items": {{
        "type": "string",
        "minLength": 2,
        "maxLength": {MAX_WORD_LENGTH}
      }}
    }}
  }}
}}"#
    )
}

fn extension_review_schema_json() -> String {
    format!(
        r#"{{
  "type": "object",
  "additionalProperties": false,
  "required": ["approved", "can_extend", "reason", "words"],
  "properties": {{
    "approved": {{ "type": "boolean" }},
    "can_extend": {{ "type": "boolean" }},
    "reason": {{ "type": "string", "minLength": 1, "maxLength": 300 }},
    "words": {{
      "type": "array",
      "maxItems": {MAX_EXTENSION_WORDS},
      "items": {{
        "type": "string",
        "minLength": 2,
        "maxLength": {MAX_WORD_LENGTH}
      }}
    }}
  }}
}}"#
    )
}

#[cfg(test)]
mod fallback_tests {
    use super::*;
    use std::sync::Mutex;

    fn alpha_suffix(mut index: usize) -> String {
        let mut chars = Vec::new();
        index += 1;
        while index > 0 {
            let remainder = (index - 1) % 26;
            chars.push((b'A' + remainder as u8) as char);
            index = (index - 1) / 26;
        }
        chars.iter().rev().collect()
    }

    fn sample_words(prefix: &str, count: usize) -> Vec<String> {
        (0..count)
            .map(|index| format!("{prefix}{}", alpha_suffix(index)))
            .collect()
    }

    #[test]
    fn generation_review_rejects_theme_drift() {
        let generated = GeneratedWordBank {
            name: "Home Renovation".into(),
            can_extend: true,
            words: sample_words("RENO", MIN_READY_WORD_COUNT),
        };
        let review = GenerationReview {
            approved: false,
            can_extend: false,
            reason: "Most words fit animals better than renovation.".into(),
            words: sample_words("ANIMAL", 12),
        };

        let error = apply_generation_review(generated, review).unwrap_err();
        assert!(error.contains("theme drift"));
    }

    #[test]
    fn generation_review_requires_minimum_approved_words() {
        let generated = GeneratedWordBank {
            name: "Home Renovation".into(),
            can_extend: true,
            words: sample_words("RENO", GENERATED_WORD_COUNT),
        };
        let review = GenerationReview {
            approved: true,
            can_extend: true,
            reason: "Too many words drifted off topic.".into(),
            words: sample_words("RENOVATE", MIN_READY_WORD_COUNT - 1),
        };

        let error = apply_generation_review(generated, review).unwrap_err();
        assert!(error.contains("survived theme review"));
    }

    #[test]
    fn generation_review_filters_words_and_can_disable_extension() {
        let generated = GeneratedWordBank {
            name: "Home Renovation".into(),
            can_extend: true,
            words: sample_words("RENO", GENERATED_WORD_COUNT),
        };
        let mut reviewed_words = sample_words("TOOLS", MIN_READY_WORD_COUNT);
        reviewed_words.push("toolsa".into());
        reviewed_words.push("tile-adhesive".into());

        let review = GenerationReview {
            approved: true,
            can_extend: false,
            reason: "Kept only strong renovation words.".into(),
            words: reviewed_words,
        };

        let reviewed = apply_generation_review(generated, review).unwrap();
        assert_eq!(reviewed.words.len(), MIN_READY_WORD_COUNT + 1);
        assert!(reviewed.words.contains(&"TILEADHESIVE".into()));
        assert!(!reviewed.can_extend);
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test]
    async fn custom_generation_requires_external_generator_when_generator_is_missing() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var("WORD_BANK_GENERATOR_BIN").ok();
        std::env::set_var("WORD_BANK_GENERATOR_BIN", "/definitely/missing/codex");

        let error = run_generation("ocean animals").await.unwrap_err();
        assert!(error.contains("theme-accurate words"));

        if let Some(value) = previous {
            std::env::set_var("WORD_BANK_GENERATOR_BIN", value);
        } else {
            std::env::remove_var("WORD_BANK_GENERATOR_BIN");
        }
    }

    #[tokio::test]
    async fn built_in_extension_requires_external_generator_when_generator_is_missing() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var("WORD_BANK_GENERATOR_BIN").ok();
        std::env::set_var("WORD_BANK_GENERATOR_BIN", "/definitely/missing/codex");

        let existing_words = vec![
            "OCEAN".to_string(),
            "WAVES".to_string(),
            "TURTLE".to_string(),
            "DOLPHIN".to_string(),
            "SEAHORSE".to_string(),
        ];
        let error = run_extension("Ocean Pals", "friendly ocean adventure", &existing_words)
            .await
            .unwrap_err();

        assert!(error.contains("theme-accurate words"));

        if let Some(value) = previous {
            std::env::set_var("WORD_BANK_GENERATOR_BIN", value);
        } else {
            std::env::remove_var("WORD_BANK_GENERATOR_BIN");
        }
    }

    #[tokio::test]
    async fn custom_extension_requires_external_generator_when_generator_is_missing() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var("WORD_BANK_GENERATOR_BIN").ok();
        std::env::set_var("WORD_BANK_GENERATOR_BIN", "/definitely/missing/codex");

        let existing_words = vec![
            "OCEAN".to_string(),
            "WAVES".to_string(),
            "TURTLE".to_string(),
            "DOLPHIN".to_string(),
            "SEAHORSE".to_string(),
        ];
        let error = run_custom_extension("Ocean Pals", "friendly ocean adventure", &existing_words)
            .await
            .unwrap_err();

        assert!(error.contains("theme-accurate words"));

        if let Some(value) = previous {
            std::env::set_var("WORD_BANK_GENERATOR_BIN", value);
        } else {
            std::env::remove_var("WORD_BANK_GENERATOR_BIN");
        }
    }
}

pub fn weighted_custom_word_sample(
    words: &[String],
    usage_by_word: &HashMap<String, (i64, Option<String>)>,
    count: usize,
) -> Vec<String> {
    weighted_word_sample(words, usage_by_word, count)
}

pub fn weighted_word_sample(
    words: &[String],
    usage_by_word: &HashMap<String, (i64, Option<String>)>,
    count: usize,
) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let now = Utc::now();
    let mut remaining: Vec<String> = words.to_vec();
    let mut selected = Vec::new();

    while selected.len() < count && !remaining.is_empty() {
        let total_weight: f64 = remaining
            .iter()
            .map(|word| {
                let (times_used, last_used_at) =
                    usage_by_word.get(word).cloned().unwrap_or((0, None));
                custom_word_weight(times_used, last_used_at.as_deref(), now)
            })
            .sum();

        if total_weight <= f64::EPSILON {
            break;
        }

        let mut threshold = rng.gen_range(0.0..total_weight);
        let mut chosen_index = 0usize;

        for (index, word) in remaining.iter().enumerate() {
            let (times_used, last_used_at) = usage_by_word.get(word).cloned().unwrap_or((0, None));
            threshold -= custom_word_weight(times_used, last_used_at.as_deref(), now);
            if threshold <= 0.0 {
                chosen_index = index;
                break;
            }
        }

        selected.push(remaining.swap_remove(chosen_index));
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_or_frequent_words_have_lower_weight() {
        let now = Utc::now();
        let fresh = custom_word_weight(0, None, now);
        let frequent = custom_word_weight(4, None, now);
        let recent = custom_word_weight(0, Some(&now.to_rfc3339()), now);

        assert!(frequent < fresh);
        assert!(recent < fresh);
    }

    #[test]
    fn normalize_words_filters_duplicates_and_invalid_entries() {
        let words = vec![
            "beach".to_string(),
            "beach".to_string(),
            "sea-shell".to_string(),
            "123".to_string(),
            "superextraordinarylongword".to_string(),
        ];

        let normalized = normalize_words(&words, &["OCEAN".to_string()]);

        assert_eq!(
            normalized,
            vec!["BEACH".to_string(), "SEASHELL".to_string()]
        );
    }

    #[test]
    fn weighted_sample_returns_unique_words_up_to_requested_count() {
        let words = vec![
            "OCEAN".to_string(),
            "WAVES".to_string(),
            "TURTLE".to_string(),
            "CORAL".to_string(),
        ];

        let usage_by_word = HashMap::from([
            ("OCEAN".to_string(), (3, None)),
            ("WAVES".to_string(), (0, None)),
        ]);

        let selected = weighted_custom_word_sample(&words, &usage_by_word, 3);
        let unique: HashSet<_> = selected.iter().collect();

        assert_eq!(selected.len(), 3);
        assert_eq!(unique.len(), 3);
    }
}
