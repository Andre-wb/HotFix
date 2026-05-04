/// Two-factor authentication
/// Generating code and verifying email

use chrono::Utc;
use sha2::{Sha256, Digest};
use crate::schemas::{CodeVerificationResult, EmailVerificationCode};
use rand::prelude::*;
use uuid::Uuid;
use crate::db::DbPool;

fn hash_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_code() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0..1_000_000))
}

pub async fn create_verification_code(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<String, sqlx::Error> {
    let code = generate_code();
    let code_hash = hash_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(15);

    sqlx::query("UPDATE email_verification_codes SET used = TRUE WHERE user_id = $1 AND used = FALSE")
        .bind(user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO email_verification_codes (user_id, code_hash, expires_at)
         VALUES ($1, $2, $3)"
    ).bind(user_id).bind(&code_hash).bind(expires_at).execute(pool).await?;

    Ok(code)
}

pub async fn verify_code(
    pool: &DbPool,
    user_id: Uuid,
    submitted_code: &str,
) -> Result<CodeVerificationResult, sqlx::Error> {
    let record = sqlx::query_as::<_, EmailVerificationCode>(
        "SELECT * FROM email_verification_codes
         WHERE user_id = $1 AND used = FALSE
         ORDER BY created_at DESC
         LIMIT 1"
    ).bind(user_id).fetch_optional(pool).await?;

    let record = match record {
        Some(record) => record,
        None => return Ok(CodeVerificationResult::Invalid),
    };

    if record.attempts >= 5 {
        return Ok(CodeVerificationResult::TooManyAttempts);
    }

    sqlx::query("UPDATE email_verification_codes SET attempts = attempts + 1 WHERE id = $1")
        .bind(record.id)
        .execute(pool)
        .await?;


    if record.expires_at < Utc::now() {
        return Ok(CodeVerificationResult::Expired);
    }

    if record.code_hash != hash_code(submitted_code) {
        return Ok(CodeVerificationResult::Invalid);
    }

    sqlx::query("UPDATE email_verification_codes SET used = TRUE WHERE id = $1")
        .bind(record.id)
        .execute(pool)
        .await?;

    Ok(CodeVerificationResult::Valid)
}

pub async fn mark_email_verified(pool: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_last_login(pool: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}