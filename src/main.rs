/// Main enter point

use hotfix::create_router;
pub mod db;
pub mod config;
pub mod routes;
pub mod schemas;



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

    let app = create_router(pool).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}