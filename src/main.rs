use axum::{
    Router,
    routing::{get, post},
    response::{Html, Redirect},
    extract::State,
    Form,
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
use aes_gcm::{aead::{Aead, AeadCore, KeyInit, OsRng as AesRng}, Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine, };
use serde::{Serialize, Deserialize};


mod db;
mod config;


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

#[derive(Debug)]
struct RegisterFormError {
    field: String,
    message: String,
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

async fn get_register(flash_message: Option<String>) -> Html<String> {
    Html(RegisterTemplate { flash_message }.render().unwrap())
}

async fn get_login(flash_message: Option<String>) -> Html<String> {
    Html(LoginTemplate { flash_message }.render().unwrap())
}

async fn get_profile(flash_message: Option<String>) -> Html<String> {
    Html(ProfileTemplate.render().unwrap())
}

async fn get_problems(flash_message: Option<String>) -> Html<String> {
    Html(ProblemsTemplate.render().unwrap())
}

fn validate_registration(form: &RegisterForm) -> Result<(), String> {
    if form.username.len() < 3 {
        return Err("Username must be at least 3 characters".to_string());
    }
    if form.username.len() > 30 {
        return Err("Username must be less than 30 characters".to_string());
    }
    if !form.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Username can only contain letters, numbers, and underscores".to_string());
    }

    if !form.email.contains('@') || !form.email.contains('.') {
        return Err("Please enter a valid email address".to_string());
    }

    if form.password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    if !form.password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }
    if !form.password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }
    if !form.password.chars().any(|c| c.is_numeric()) {
        return Err("Password must contain at least one number".to_string());
    }

    if form.password != form.confirm_password {
        return Err("Passwords do not match".to_string());
    }

    Ok(())
}


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
        .route("/register", get(get_register))
        .route("/login", get(get_login))
        .route("/profile", get(get_profile))
        .route("/problems", get(get_problems));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}