use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub type DbPool = PgPool;

// Add FromRow here - this is what was missing!
#[derive(Debug, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Initialize database connection pool
pub async fn init_pool() -> DbPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool")
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

/// Create a new user
pub async fn create_user(
    pool: &DbPool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let query = "
        INSERT INTO users (username, email, password_hash, created_at)
        VALUES ($1, $2, $3, NOW())
        RETURNING id, username, email, password_hash, created_at
    ";

    let user = sqlx::query_as::<_, User>(query)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;

    Ok(user)
}