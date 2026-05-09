/// Main entry point

use hotfix::create_router;
use hotfix::db;
use hotfix::config;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use time::Duration;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    if let Err(error) = config::Config::init() {
        eprintln!("Failed to initialize configuration: {error}");
        std::process::exit(1);
    }

    let config = config::Config::global();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.into())
        )
        .init();

    let pool = db::init_pool().await;

    let session_store = PostgresStore::new(pool.clone());
    if let Err(e) = session_store.migrate().await {
        tracing::error!("Failed to migrate session store: {}", e);
        std::process::exit(1);
    }

    let same_site = if config.is_production() {
        tower_sessions::cookie::SameSite::Strict
    } else {
        tower_sessions::cookie::SameSite::Lax
    };

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_http_only(true)
        .with_same_site(same_site)
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let app = create_router(pool).await.layer(session_layer);

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:8000").await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind to address: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("listening on {}", listener.local_addr().unwrap());

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}