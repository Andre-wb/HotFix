use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqlitePoolOptions}};
use std::str::FromStr;
use tracing::info;

pub async fn init_pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite://app.db")
        .expect("invalid db url")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .expect("failed to connect to database");

    run_migrations(&pool).await;
    info!("database ready");
    pool
}

async fn run_migrations(pool: &SqlitePool) {
    // Run all table creation in a single transaction —
    // if any fails, none are created (all or nothing)
    let mut tx = pool.begin().await.expect("failed to begin transaction");

    // ── Users ────────────────────────────────────────────────────────────────
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id            INTEGER  PRIMARY KEY AUTOINCREMENT,
            username_hash TEXT     NOT NULL UNIQUE,
            username_enc  TEXT     NOT NULL,
            password_hash TEXT     NOT NULL,
            rank          TEXT     NOT NULL,
            totp_secret   TEXT,
            totp_enabled  BOOLEAN  NOT NULL DEFAULT 0,
            total_points  INTEGER  NOT NULL DEFAULT 0,
            tasks_solved  INTEGER  NOT NULL DEFAULT 0,
            created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create users table");

    // ── Sessions ─────────────────────────────────────────────────────────────
    // Used by tower-sessions-sqlx-store — schema it expects
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tower_sessions (
            id          TEXT     PRIMARY KEY NOT NULL,
            data        BLOB     NOT NULL,
            expiry_date INTEGER  NOT NULL
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create sessions table");

    // ── Tasks ────────────────────────────────────────────────────────────────
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id            INTEGER  PRIMARY KEY AUTOINCREMENT,
            language      TEXT     NOT NULL,
            difficulty    TEXT     NOT NULL,
            type          TEXT     NOT NULL,
            title         TEXT     NOT NULL UNIQUE,
            description   TEXT     NOT NULL,
            broken_code   TEXT     NOT NULL,
            correct_code  TEXT     NOT NULL,
            tags          JSON     NOT NULL,
            hint          TEXT     NOT NULL,
            time_limit    INTEGER  NOT NULL,
            active        BOOLEAN  NOT NULL DEFAULT 1,
            created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create tasks table");

    // ── Task test cases ───────────────────────────────────────────────────────
    // Each task can have multiple input/output pairs for Tier-2 validation
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS task_tests (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id   INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            input     TEXT    NOT NULL,
            expected  TEXT    NOT NULL
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create task_tests table");

    // ── Submissions ───────────────────────────────────────────────────────────
    // Saved only for logged-in users — guests get no history
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS submissions (
            id           INTEGER  PRIMARY KEY AUTOINCREMENT,
            user_id      INTEGER  NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            task_id      INTEGER  NOT NULL REFERENCES tasks(id),
            submitted    TEXT     NOT NULL,
            is_correct   BOOLEAN  NOT NULL,
            tier_used    TEXT     NOT NULL,
            time_taken   INTEGER  NOT NULL,
            hint_used    BOOLEAN  NOT NULL DEFAULT 0,
            points       INTEGER  NOT NULL DEFAULT 0,
            created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create submissions table");

    // ── Leaderboard view ──────────────────────────────────────────────────────
    // Virtual — computed from users table, but we create a view for easy queries
    sqlx::query(
        "CREATE VIEW IF NOT EXISTS leaderboard AS
            SELECT
                id,
                username_enc,
                total_points,
                tasks_solved,
                rank,
            FROM users
            ORDER BY total_points DESC
            LIMIT 100"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create leaderboard view");

    // ── Per-language stats ────────────────────────────────────────────────────
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_language_stats (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            language     TEXT    NOT NULL,
            solved       INTEGER NOT NULL DEFAULT 0,
            attempted    INTEGER NOT NULL DEFAULT 0,
            total_points INTEGER NOT NULL DEFAULT 0,
            UNIQUE(user_id, language)   -- one row per user per language
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create user_language_stats table");

    // ── Streaks ───────────────────────────────────────────────────────────────
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS streaks (
            user_id        INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            current_streak INTEGER  NOT NULL DEFAULT 0,
            longest_streak INTEGER  NOT NULL DEFAULT 0,
            last_solve     DATETIME          -- date of last correct submission
        )"
    )
        .execute(&mut *tx)
        .await
        .expect("failed to create streaks table");

    // ── Indexes ───────────────────────────────────────────────────────────────
    // Speed up the most common queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_lang_diff
                 ON tasks(language, difficulty) WHERE active = 1")
        .execute(&mut *tx).await.expect("idx_tasks_lang_diff");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_submissions_user
                 ON submissions(user_id, created_at DESC)")
        .execute(&mut *tx).await.expect("idx_submissions_user");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_submissions_task
                 ON submissions(task_id)")
        .execute(&mut *tx).await.expect("idx_submissions_task");

    tx.commit().await.expect("failed to commit migrations");
    info!("migrations complete");
}