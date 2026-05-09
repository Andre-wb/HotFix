/// GET and POST routes

use axum::{
    response::{Html, Redirect},
    extract::State,
    Form,
};
use askama::Template;
use chrono::Utc;
use tower_sessions::Session;
use uuid::Uuid;
use crate::email;
use crate::db::{self, verify_password, hash_password};
use crate::schemas::{
    LoginTemplate,
    ProblemsTemplate,
    ProfileTemplate,
    RegisterForm,
    RegisterTemplate,
    LoginForm,
    CodeVerificationResult,
    TwoFaForm,
    TwoFaTemplate
};
use crate::auth::{create_verification_code, verify_code, mark_email_verified, update_last_login};

const SESSION_PENDING_USER: &str = "pending_2fa_user_id";
const SESSION_PENDING_TYPE: &str = "pending_2fa_type";
const SESSION_USER_ID: &str = "user_id";

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

pub async fn get_2fa(session: Session) -> Result<Html<String>, Redirect> {
    if session.get::<String>(SESSION_PENDING_USER).await.ok().flatten().is_none() {
        return Err(Redirect::to("/login"));
    }
    Ok(Html(TwoFaTemplate { flash_message: None }.render().unwrap()))
}

pub async fn post_register(
    State(pool): State<db::DbPool>,
    session: Session,
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
        Err(error) => {
            tracing::error!("Database error checking user: {}", error);
            let error_html = RegisterTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
        _ => {}
    }

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(error) => {
            tracing::error!("Failed to create password hash: {}", error);
            let error_html = RegisterTemplate {
                flash_message: Some("Error while processing request".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let user = match db::create_user(&pool, &form.username, &form.email, &password_hash).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!("Failed to create user: {}", error);
            let error_html = RegisterTemplate {
                flash_message: Some("Error while creating account".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let code = match create_verification_code(&pool, user.id).await {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("Code gen error: {}", e);
            let error_html = RegisterTemplate {
                flash_message: Some("Error while generating verification code".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let email_addr = user.email.clone();
    let code_clone = code.clone();
    tokio::spawn(async move {
        if let Err(error) = email::send_verification_code(&email_addr, &code_clone).await {
            tracing::error!("Background email during register failed: {}", error);
        }
    });

    session.insert(SESSION_PENDING_USER, user.id.to_string()).await.ok();
    session.insert(SESSION_PENDING_TYPE, "registration").await.ok();

    Ok(Redirect::to("/2fa_confirm"))
}

pub async fn post_login(
    State(pool): State<db::DbPool>,
    session: Session,
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
            tracing::error!("Database error: {}", error);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    match verify_password(&form.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            let error_html = LoginTemplate {
                flash_message: Some("Invalid username/email or password".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Err(error) => {
            tracing::error!("Password verification error: {}", error);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    }

    if !user.email_verified {
        let code = match create_verification_code(&pool, user.id).await {
            Ok(code) => code,
            Err(error) => {
                tracing::error!("Failed to create verification code: {}", error);
                let error_html = LoginTemplate {
                    flash_message: Some("Error sending verification code. Please try again.".to_string())
                }.render().unwrap();
                return Err(Html(error_html));
            }
        };

        let email_addr = user.email.clone();
        let code_clone = code.clone();
        tokio::spawn(async move {
            if let Err(error) = email::send_verification_code(&email_addr, &code_clone).await {
                tracing::error!("Background email during login failed: {}", error);
            }
        });

        session.insert(SESSION_PENDING_USER, user.id.to_string()).await.ok();
        session.insert(SESSION_PENDING_TYPE, "registration").await.ok();

        return Ok(Redirect::to("/2fa_confirm"));
    }

    let needs_2fa = user.last_login_at
        .map(|time| Utc::now() - time > chrono::Duration::days(30))
        .unwrap_or(true);

    if needs_2fa {
        let code = match create_verification_code(&pool, user.id).await {
            Ok(code) => code,
            Err(error) => {
                tracing::error!("Failed to create verification code: {}", error);
                let error_html = LoginTemplate {
                    flash_message: Some("Failed to send verification email. Please try again".to_string())
                }.render().unwrap();
                return Err(Html(error_html));
            }
        };

        let email_addr = user.email.clone();
        let code_clone = code.clone();
        tokio::spawn(async move {
            if let Err(e) = email::send_verification_code(&email_addr, &code_clone).await {
                tracing::error!("Background email during login failed: {}", e);
            }
        });

        session.insert(SESSION_PENDING_USER, user.id.to_string()).await.ok();
        session.insert(SESSION_PENDING_TYPE, "login").await.ok();

        Ok(Redirect::to("/2fa_confirm"))
    } else {
        if let Err(error) = complete_login(&pool, &session, user.id).await {
            tracing::error!("Error completing authentication: {}", error);
            let error_html = TwoFaTemplate {
                flash_message: Some("Error while completing authentication. Please try again".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Ok(Redirect::to("/problems"))
    }
}

pub async fn post_2fa(
    State(pool): State<db::DbPool>,
    session: Session,
    Form(form): Form<TwoFaForm>,
) -> Result<Redirect, Html<String>> {
    let user_id_str = match session.get::<String>(SESSION_PENDING_USER).await.ok().flatten() {
        Some(id) => id,
        None => {
            tracing::warn!("2FA attempt with expired session");
            let error_html = TwoFaTemplate {
                flash_message: Some("Your session expired. Please sign in again".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let user_id = match Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(error) => {
            tracing::error!("Invalid UUID in session: {}", error);
            let error_html = TwoFaTemplate {
                flash_message: Some("Session error. Please sign in again".to_string())
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let verification_type = session.get::<String>(SESSION_PENDING_TYPE).await.ok().flatten().unwrap_or_default();

    match verify_code(&pool, user_id, &form.confirm_code).await {
        Ok(CodeVerificationResult::Valid) => {
            session.remove::<String>(SESSION_PENDING_USER).await.ok();
            session.remove::<String>(SESSION_PENDING_TYPE).await.ok();

            if verification_type == "registration" {
                if let Err(error) = mark_email_verified(&pool, user_id).await {
                    tracing::error!("Failed to mark email verified: {}", error);
                    let error_html = TwoFaTemplate {
                        flash_message: Some("Error while verifying code. Please try again".to_string())
                    }.render().unwrap();
                    return Err(Html(error_html));
                }
            }

            if let Err(error) = complete_login(&pool, &session, user_id).await {
                tracing::error!("Failed to complete authentication: {}", error);
                let error_html = TwoFaTemplate {
                    flash_message: Some("Error completing authentication. Please try again".to_string())
                }.render().unwrap();
                return Err(Html(error_html));
            }

            Ok(Redirect::to("/problems"))
        }
        Ok(CodeVerificationResult::Invalid) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Incorrect code".to_string())
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::Expired) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Code expired. Please go back and request a new one".to_string())
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::TooManyAttempts) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Too many failed attempts. Please request a new code".to_string())
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::AlreadyUsed) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Code already used".to_string())
            }.render().unwrap()))
        }
        Err(error) => {
            tracing::error!("Failed to verify code: {}", error);
            Err(Html(TwoFaTemplate {
                flash_message: Some("An error occurred".to_string())
            }.render().unwrap()))
        }
    }
}

async fn complete_login(
    pool: &db::DbPool,
    session: &Session,
    user_id: Uuid,
) -> Result<(), String> {
    update_last_login(pool, user_id).await
        .map_err(|e| e.to_string())?;
    session.insert(SESSION_USER_ID, user_id.to_string()).await
        .map_err(|e| e.to_string())?;
    Ok(())
}