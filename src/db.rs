use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::migrate::Migrator;
use crate::RegisterForm;
use serde_json::Value;
use serde::{Deserialize, Serialize};

pub type DbPool = PgPool;
pub static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Debug, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub rank: String,
    pub problems_solved: i32, // for tracking how many problems user solved
    pub tags: Value, // for tracking topics of problems which user solved and how many problems user solved in 1 exact topic
}

#[derive(Debug, FromRow)]
pub struct Problems {
    pub id: Uuid,
    pub name: String,
    pub topics: Vec<String>,
    pub language: String,
    pub difficulty: Difficulty,
    /// Problem have incorrect code, which user need to fix and description about what correct code should do
    /// User's attempts checks with tests
    /// User's code have to pass all the test just like correct version to mark as solved
    pub correct_version: String,
    pub incorrect_version: String,
    pub tests: Value, // to test which output we get from program with particular input
    pub time_limit_seconds: i32, // how much time users have to solve the problem
    pub description: String,
    pub solved_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "difficulty_enum", rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Easy => write!(f, "easy"),
            Difficulty::Medium => write!(f, "medium"),
            Difficulty::Hard => write!(f, "hard"),
        }
    }
}


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

/// Check if username or email already exists
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
        INSERT INTO users (username, email, password_hash, created_at, rank, problems_solved, tags)
        VALUES ($1, $2, $3, NOW(), 'beginner', 0, '{}'::jsonb)
        RETURNING id, username, email, password_hash, created_at, rank, problems_solved, tags
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
    // FIXED: Added space after users
    let query = "
        SELECT * FROM users
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