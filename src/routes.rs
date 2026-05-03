/// GET and POST routes

use axum::{
    response::{Html, Redirect},
    extract::State,
    Form,
};
use askama::Template;
use crate::db::{self, verify_password, hash_password};
use crate::schemas::{LoginTemplate, ProblemsTemplate, ProfileTemplate, RegisterForm, RegisterTemplate, LoginForm};


pub async fn get_register(flash_message: Option<String>) -> Html<String> {
    Html(RegisterTemplate { flash_message }.render().unwrap())
}

pub async fn get_login(flash_message: Option<String>) -> Html<String> {
    Html(LoginTemplate { flash_message }.render().unwrap())
}

pub async fn get_profile(_flash_message: Option<String>) -> Html<String> {
    Html(ProfileTemplate.render().unwrap())
}

pub async fn get_problems(_flash_message: Option<String>) -> Html<String> {
    Html(ProblemsTemplate.render().unwrap())
}

pub async fn post_register(
    State(pool): State<db::DbPool>,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, Html<String>> {
    if let Err(error_message) = db::validate_registration(&form) {
        let error_html = RegisterTemplate {
            flash_message: Some(error_message)
        }.render().unwrap();
        return Err(Html(error_html));
    }

    match db::user_exists(&pool, &form.username, &form.email).await {
        Ok(true) => {
            let error_html = RegisterTemplate {
                flash_message: Some("Username or email already exists".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Err(e) => {
            eprintln!("Database error checking user: {}", e);
            let error_html = RegisterTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
        _ => {}
    }

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(e) => {
            let error_html = RegisterTemplate {
                flash_message: Some(format!("Error creating account: {}", e))
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    match db::create_user(&pool, &form.username, &form.email, &password_hash).await {
        Ok(_) => {
            Ok(Redirect::to("/problems"))
        }
        Err(e) => {
            eprintln!("Failed to create user: {}", e);
            let error_html = RegisterTemplate {
                flash_message: Some("Failed to create account. Please try again.".to_string())
            }.render().unwrap();
            Err(Html(error_html))
        }
    }
}

pub async fn post_login(
    State(pool): State<db::DbPool>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, Html<String>> {
    let user = match db::get_user_by_identifier(&pool, &form.identifier).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let error_html = LoginTemplate {
                flash_message: Some("Invalid username/email or password".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        },
        Err(error) => {
            eprintln!("Database error: {error}");
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    match verify_password(&form.password, &user.password_hash) {
        Ok(true) => {
            Ok(Redirect::to("/problems"))
        }
        Ok(false) => {
            let error_html = LoginTemplate {
                flash_message: Some("Invalid username/email or password".to_string())
            }.render().unwrap();
            Err(Html(error_html))
        }
        Err(e) => {
            eprintln!("Password verification error: {}", e);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string())
            }.render().unwrap();
            Err(Html(error_html))
        }
    }
}