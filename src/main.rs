use axum::{
    Router,
    routing::{get, post},
};
use askama::Template;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash,
        SaltString,
        PasswordVerifier,
        PasswordHasher,
    },
    Argon2,
};
use serde::Deserialize;

mod db;
mod config;
mod routes;
use routes::{get_problems, get_profile, get_login, post_login, get_register, post_register};

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    flash_message: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    flash_message: Option<String>,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate;

#[derive(Template)]
#[template(path = "problems.html")]
struct ProblemsTemplate;

#[derive(Deserialize, Debug)]
struct RegisterForm {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
}

#[derive(Deserialize, Debug)]
struct LoginForm {
    identifier: String,
    password: String,
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| format!("Hashing failed: {error}"))?
        .to_string();

    Ok(password_hash)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|error| format!("Invalid hash: {error}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
    )
}


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