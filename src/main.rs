mod db;

use axum::{Router, routing::get};
use askama::Template;

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    flash_message: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate;

#[derive(Template)]
#[template(path = "main.html")]
struct MainTemplate;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .init();

    let pool = db::init_pool().await;

    let app = Router::new()
        .route("/", get(|| async { "ok" })); 

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::info!("listening on http://127.0.0.1:8000");
    axum::serve(listener, app).await.unwrap();
}