/// Main enter point

use hotfix::create_router;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use time::Duration;
pub mod db;
pub mod config;
pub mod routes;
pub mod schemas;
pub mod email;
pub mod auth;



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

    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await.unwrap();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let app = create_router(pool).await.layer(session_layer);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}