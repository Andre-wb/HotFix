/// Library for integration tests

pub mod config;
pub mod db;
pub mod routes;
pub mod schemas;
pub mod email;
pub mod auth;
pub mod ai;
pub mod sandbox;
pub mod problem_validation;

pub use config::Config;
pub use db::DbPool;
pub use routes::*;
pub use schemas::*;
pub use auth::*;

use tower_http::services::ServeDir;

pub async fn create_router(pool: DbPool) -> axum::Router {
    use axum::{Router, routing::{get, post}};
    use routes::{
        get_problems, get_problem, post_submit, post_generate_problem,
        get_profile, get_login, post_login,
        get_register, post_register, get_2fa, post_2fa
    };

    Router::new()
        .route("/register", get(get_register))
        .route("/register", post(post_register))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/problems", get(get_problems))
        .route("/problems/:id", get(get_problem))
        .route("/problems/:id/submit", post(post_submit))
        .route("/admin/generate", post(post_generate_problem))
        .route("/2fa_confirm", get(get_2fa))
        .route("/2fa_confirm", post(post_2fa))
        .route("/profile", get(get_profile))
        .route("/logout", get(logout))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(pool)
}