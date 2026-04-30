use axum::{
    Router,
    routing::{
        get, post
    }
};
use askama::Template;

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    flash_message: String,
}

#[derive(Template)]
#[template(path = "Login.html")]
struct LoginTemplate;

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate;
#[derive(Template)]
#[template(path = "main.html")]
struct MainTemplate;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
