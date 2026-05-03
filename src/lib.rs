pub mod config;
pub mod db;
pub mod routes;
pub mod schemas;

pub use config::Config;
pub use db::DbPool;
pub use routes::*;
pub use schemas::*;

pub async fn create_router(pool: DbPool) -> axum::Router {
    use axum::{Router, routing::{get, post}};
    use routes::{
        get_problems, get_profile, get_login, post_login,
        get_register, post_register
    };

    Router::new()
        .route("/register", get(get_register))
        .route("/register", post(post_register))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/profile", get(get_profile))
        .route("/problems", get(get_problems))
        .with_state(pool)
}