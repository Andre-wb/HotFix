/// Main enter point

use axum::{
    Router,
    routing::{get, post},
};
mod db;
mod config;
mod routes;
use routes::{get_problems, get_profile, get_login, post_login, get_register, post_register};
mod schemas;



#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    if let Err(error) = config::Config::init() {
        eprintln!("Failed to initialize configuration: {error}");
        std::process::exit(1);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .init();

    let pool = db::init_pool().await;

    let app = Router::new()
        .route("/register", get(get_register))
        .route("/register", post(post_register))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/profile", get(get_profile))
        .route("/problems", get(get_problems))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}