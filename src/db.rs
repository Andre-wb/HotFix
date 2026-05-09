/// Database initialisation and utils

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash,
        SaltString,
        PasswordVerifier,
        PasswordHasher,
    },
    Argon2,
};
use sqlx::{PgPool, migrate::Migrator};
use crate::schemas::{RegisterForm, User};


pub type DbPool = PgPool;
pub static MIGRATOR: Migrator = sqlx::migrate!();



/// Initialize database connection pool
pub async fn init_pool() -> DbPool {
    let database_url = crate::config::database_url();
    println!("Connecting to {}", database_url);

    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed to create database connection pool");
    println!("Running migrations");

    MIGRATOR.run(&pool)
        .await
        .expect("Failed to run migrations");

    println!("Database ready");
    pool
}

pub async fn user_exists(
    pool: &DbPool,
    username: &str,
    email: &str
) -> Result<bool, sqlx::Error> {
    let query = "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 OR email = $2)";

    let (exists,): (bool,) = sqlx::query_as(query)
        .bind(username)
        .bind(email)
        .fetch_one(pool)
        .await?;

    Ok(exists)
}

pub async fn create_user(
    pool: &DbPool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let query = "
        INSERT INTO users (username, email, password_hash, created_at, rank, problems_solved, tags, email_verified)
        VALUES ($1, $2, $3, NOW(), 'beginner', 0, '{}'::jsonb, FALSE)
        RETURNING id, username, email, password_hash, created_at, rank, problems_solved, tags, last_login_at, email_verified
    ";

    let user = sqlx::query_as::<_, User>(query)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;

    Ok(user)
}

pub async fn get_user_by_identifier(
    pool: &DbPool,
    identifier: &str,
) -> Result<Option<User>, sqlx::Error> {
    let query = "
        SELECT id, username, email, password_hash, created_at, rank,
               problems_solved, tags, last_login_at, email_verified
        FROM users
        WHERE username = $1 OR email = $1
        LIMIT 1
    ";

    let user = sqlx::query_as::<_, User>(query)
        .bind(identifier)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub fn validate_registration(form: &RegisterForm) -> Result<(), String> {
    if form.username.len() < 3 {
        return Err("Username must be at least 3 characters".to_string());
    }
    if form.username.len() > 30 {
        return Err("Username must be less than 30 characters".to_string());
    }
    if !form.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username can only contain letters, numbers, and underscores".to_string());
    }

    if !form.email.contains('@') || !form.email.contains('.') {
        return Err("Please enter a valid email address".to_string());
    }

    if form.password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    if !form.password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }
    if !form.password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }
    if !form.password.chars().any(|c| c.is_numeric()) {
        return Err("Password must contain at least one number".to_string());
    }

    if form.password != form.confirm_password {
        return Err("Passwords do not match".to_string());
    }

    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| format!("Hashing failed: {error}"))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|error| format!("Invalid hash: {error}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
    )
}