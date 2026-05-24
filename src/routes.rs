/// GET and POST routes

use axum::{
    response::{Html, Redirect},
    extract::{State, Path},
    Form,
    http::{HeaderMap, StatusCode},
    Json
};
use askama::Template;
use chrono::Utc;
use serde_json::{json, Value};
use tower_sessions::Session;
use uuid::Uuid;
use crate::{email};
use crate::db::{self, verify_password, hash_password, DbPool};
use crate::schemas::{
    LoginTemplate,
    ProblemsTemplate,
    UserProfileTemplate,
    RegisterForm,
    RegisterTemplate,
    LoginForm,
    CodeVerificationResult,
    TwoFaForm,
    TwoFaTemplate,
    ProblemTemplate,
    AiService,
    TestResult,
    TestCase,
    SubmitForm,
    Difficulty,
    ResultsTemplate,
};
use crate::sandbox::SandBox;
use crate::auth::{create_verification_code, verify_code, mark_email_verified, update_last_login};
use crate::problem_validation::validate_problem;

const SESSION_PENDING_USER: &str = "pending_2fa_user_id";
const SESSION_PENDING_TYPE: &str = "pending_2fa_type";
const SESSION_USER_ID: &str = "user_id";
const SESSION_SOLVED: &str = "solved_problems";

async fn is_logged_in(session: &Session) -> bool {
    session.get::<String>(SESSION_USER_ID)
        .await
        .ok()
        .flatten()
        .is_some()
}

pub async fn get_register(session: Session) -> Html<String> {
    let logged_in = is_logged_in(&session).await;

    if logged_in {
        return Html("<script>window.location.href='/problems'</script>".to_string());
    }

    Html(RegisterTemplate {
        flash_message: None,
        username: None,
        email: None,
        password: None,
        confirm_password: None,
        logged_in: false,
    }.render().unwrap())
}

pub async fn get_login(session: Session) -> Html<String> {
    let logged_in = is_logged_in(&session).await;

    if logged_in {
        return Html("<script>window.location.href='/problems'</script>".to_string());
    }

    Html(LoginTemplate {
        flash_message: None,
        identifier: None,
        password: None,
        logged_in: false,
    }.render().unwrap())
}

pub async fn get_profile(
    session: Session,
    State(pool): State<DbPool>,
) -> Result<Html<String>, Redirect> {
    let user_id = match session.get::<String>(SESSION_USER_ID).await.ok().flatten() {
        Some(id) => id,
        None => return Err(Redirect::to("/login")),
    };

    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return Err(Redirect::to("/login")),
    };

    let user = match db::get_user_by_id(&pool, user_id).await {
        Ok(Some(user)) => user,
        _ => return Err(Redirect::to("/login")),
    };

    let stats = match db::get_user_stats(&pool, user_id).await {
        Ok(stats) => stats,
        Err(_) => return Err(Redirect::to("/problems")),
    };

    Ok(Html(UserProfileTemplate {
        user,
        stats,
        logged_in: true,
    }.render().unwrap()))
}

pub async fn logout(session: Session) -> Redirect {
    session.remove::<String>(SESSION_USER_ID).await.ok();
    Redirect::to("/login")
}

pub async fn get_problems(
    session: Session,
    State(pool): State<DbPool>,
) -> Html<String> {
    let problems = db::get_problems(&pool).await.unwrap_or_default();
    let logged_in = is_logged_in(&session).await;

    Html(ProblemsTemplate {
        problems,
        logged_in,
    }.render().unwrap())
}

pub async fn get_problem(
    session: Session,
    State(pool): State<DbPool>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, Redirect> {
    let problem = match db::get_problem(&pool, id).await {
        Ok(Some(p)) => p,
        _ => return Err(Redirect::to("/problems")),
    };

    let logged_in = is_logged_in(&session).await;

    Ok(Html(
        ProblemTemplate {
            problem,
            start_time: Utc::now().timestamp(),
            logged_in,
        }
            .render()
            .unwrap(),
    ))
}

pub async fn get_2fa(session: Session) -> Result<Html<String>, Redirect> {
    if session.get::<String>(SESSION_PENDING_USER).await.ok().flatten().is_none() {
        return Err(Redirect::to("/login"));
    }

    let logged_in = is_logged_in(&session).await;

    Ok(Html(TwoFaTemplate {
        flash_message: None,
        confirm_code: None,
        logged_in,
    }.render().unwrap()))
}

pub async fn post_register(
    State(pool): State<DbPool>,
    session: Session,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, Html<String>> {
    if let Err(error_message) = db::validate_registration(&form) {
        let error_html = RegisterTemplate {
            flash_message: Some(error_message),
            username: Some(form.username.clone()),
            email: Some(form.email.clone()),
            password: Some(form.password.clone()),
            confirm_password: Some(form.confirm_password.clone()),
            logged_in: false,
        }.render().unwrap();
        return Err(Html(error_html));
    }

    match db::user_exists(&pool, &form.username, &form.email).await {
        Ok(true) => {
            let error_html = RegisterTemplate {
                flash_message: Some("Username or email already exists".to_string()),
                username: Some(form.username.clone()),
                email: Some(form.email.clone()),
                password: Some(form.password.clone()),
                confirm_password: Some(form.confirm_password.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Err(error) => {
            tracing::error!("Database error checking user: {}", error);
            let error_html = RegisterTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string()),
                username: Some(form.username.clone()),
                email: Some(form.email.clone()),
                password: Some(form.password.clone()),
                confirm_password: Some(form.confirm_password.clone()),
                logged_in: false,
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
                flash_message: Some("Error while processing request".to_string()),
                username: Some(form.username.clone()),
                email: Some(form.email.clone()),
                password: Some(form.password.clone()),
                confirm_password: Some(form.confirm_password.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let user = match db::create_user(&pool, &form.username, &form.email, &password_hash).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!("Failed to create user: {}", error);
            let error_html = RegisterTemplate {
                flash_message: Some("Error while creating account".to_string()),
                username: Some(form.username.clone()),
                email: Some(form.email.clone()),
                password: Some(form.password.clone()),
                confirm_password: Some(form.confirm_password.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let code = match create_verification_code(&pool, user.id).await {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("Code gen error: {}", e);
            let error_html = RegisterTemplate {
                flash_message: Some("Error while generating verification code".to_string()),
                username: Some(form.username.clone()),
                email: Some(form.email.clone()),
                password: Some(form.password.clone()),
                confirm_password: Some(form.confirm_password.clone()),
                logged_in: false,
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
    State(pool): State<DbPool>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, Html<String>> {
    let user = match db::get_user_by_identifier(&pool, &form.identifier).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let error_html = LoginTemplate {
                flash_message: Some("Invalid username/email or password".to_string()),
                identifier: Some(form.identifier.clone()),
                password: Some(form.password.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        },
        Err(error) => {
            tracing::error!("Database error: {}", error);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again".to_string()),
                identifier: Some(form.identifier.clone()),
                password: Some(form.password.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    match verify_password(&form.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            let error_html = LoginTemplate {
                flash_message: Some("Invalid username/email or password".to_string()),
                identifier: Some(form.identifier.clone()),
                password: Some(form.password.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Err(error) => {
            tracing::error!("Password verification error: {}", error);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string()),
                identifier: Some(form.identifier.clone()),
                password: Some(form.password.clone()),
                logged_in: false,
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
                    flash_message: Some("Error sending verification code. Please try again.".to_string()),
                    identifier: Some(form.identifier.clone()),
                    password: Some(form.password.clone()),
                    logged_in: false,
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
                    flash_message: Some("Failed to send verification email. Please try again".to_string()),
                    identifier: Some(form.identifier.clone()),
                    password: Some(form.password.clone()),
                    logged_in: false,
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
                flash_message: Some("Error while completing authentication. Please try again".to_string()),
                confirm_code: None,
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Ok(Redirect::to("/problems"))
    }
}

pub async fn post_2fa(
    State(pool): State<DbPool>,
    session: Session,
    Form(form): Form<TwoFaForm>,
) -> Result<Redirect, Html<String>> {
    let user_id_str = match session.get::<String>(SESSION_PENDING_USER).await.ok().flatten() {
        Some(id) => id,
        None => {
            tracing::warn!("2FA attempt with expired session");
            let error_html = TwoFaTemplate {
                flash_message: Some("Your session expired. Please sign in again".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
            }.render().unwrap();
            return Err(Html(error_html));
        }
    };

    let user_id = match Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(error) => {
            tracing::error!("Invalid UUID in session: {}", error);
            let error_html = TwoFaTemplate {
                flash_message: Some("Session error. Please sign in again".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
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
                        flash_message: Some("Error while verifying code. Please try again".to_string()),
                        confirm_code: Some(form.confirm_code.clone()),
                        logged_in: false,
                    }.render().unwrap();
                    return Err(Html(error_html));
                }
            }

            if let Err(error) = complete_login(&pool, &session, user_id).await {
                tracing::error!("Failed to complete authentication: {}", error);
                let error_html = TwoFaTemplate {
                    flash_message: Some("Error completing authentication. Please try again".to_string()),
                    confirm_code: Some(form.confirm_code.clone()),
                    logged_in: false,
                }.render().unwrap();
                return Err(Html(error_html));
            }

            Ok(Redirect::to("/problems"))
        }
        Ok(CodeVerificationResult::Invalid) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Incorrect code".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::Expired) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Code expired. Please go back and request a new one".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::TooManyAttempts) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Too many failed attempts. Please request a new code".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::AlreadyUsed) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Code already used".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
            }.render().unwrap()))
        }
        Err(error) => {
            tracing::error!("Failed to verify code: {}", error);
            Err(Html(TwoFaTemplate {
                flash_message: Some("An error occurred".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
                logged_in: false,
            }.render().unwrap()))
        }
    }
}

pub async fn post_submit(
    State(pool): State<DbPool>,
    session: Session,
    Path(problem_id): Path<Uuid>,
    Form(form): Form<SubmitForm>,
) -> Result<Html<String>, Redirect> {
    let problem = match db::get_problem(&pool, problem_id).await {
        Ok(Some(p)) => p,
        _ => return Err(Redirect::to("/problems")),
    };

    let elapsed = Utc::now().timestamp() - form.start_time;
    if elapsed > problem.time_limit_seconds as i64 {
        return Ok(Html("Time limit exceeded".to_string()));
    }

    let tests: Vec<TestCase> = serde_json::from_value(problem.tests).unwrap_or_default();

    let sandbox = SandBox::new();
    let mut results = Vec::new();
    let mut passed = 0;

    for test in &tests {
        match sandbox
            .run(&problem.language, &form.code, &test.input, problem.time_limit_seconds as u64)
            .await
        {
            Ok(output) => {
                let ok = output.trim() == test.expected_output.trim();
                if ok {
                    passed += 1;
                }
                results.push(TestResult {
                    input: test.input.clone(),
                    expected: test.expected_output.clone(),
                    actual: output,
                    passed: ok,
                });
            }
            Err(e) => {
                results.push(TestResult {
                    input: test.input.clone(),
                    expected: test.expected_output.clone(),
                    actual: e.clone(),
                    passed: false,
                });
            }
        }
    }

    let all_passed = passed == tests.len() as i32;
    let status = if all_passed { "accepted" } else { "wrong_answer" };

    let user_id = session
        .get::<String>(SESSION_USER_ID).await.ok().flatten()
        .and_then(|id| Uuid::parse_str(&id).ok());

    let session_id = session.id().map(|s| s.to_string()).unwrap_or_default();
    let _ = db::save_submission(
        &pool, problem_id, user_id, &session_id,
        &form.code, passed, tests.len() as i32, status,
    ).await;

    if all_passed {
        if let Some(uid) = user_id {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM submissions
             WHERE user_id = $1 AND problem_id = $2 AND status = 'accepted'"
            )
                .bind(uid)
                .bind(problem_id)
                .fetch_one(&pool)
                .await
                .unwrap_or(0);

            println!("User {} has {} accepted submissions for problem {}", uid, count, problem_id);

            if count == 1 {
                println!("First time solving! Incrementing counts...");
                let _ = db::increment_solved_count(&pool, problem_id).await;
                let _ = sqlx::query("UPDATE users SET problems_solved = problems_solved + 1 WHERE id = $1")
                    .bind(uid)
                    .execute(&pool)
                    .await;
                let _ = db::update_user_tags(&pool, uid, &problem.topics).await;
            } else {
                println!("Already solved before! Skipping increment.");
            }
        } else {
            let solved: Vec<String> = session
                .get(SESSION_SOLVED).await.ok().flatten()
                .unwrap_or_default();

            if !solved.contains(&problem_id.to_string()) {
                let _ = db::increment_solved_count(&pool, problem_id).await;
                let mut solved = solved;
                solved.push(problem_id.to_string());
                let _ = session.insert(SESSION_SOLVED, solved).await;
            }
        }
    }

    let logged_in = is_logged_in(&session).await;

    Ok(Html(
        ResultsTemplate {
            passed,
            total: tests.len() as i32,
            all_passed,
            results,
            logged_in,
        }
            .render()
            .unwrap(),
    ))
}

pub async fn post_generate_problem(
    State(pool): State<DbPool>,
    headers: HeaderMap,
) -> Result<Redirect, (StatusCode, String)> {
    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if admin_token.is_empty() || provided != admin_token {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    let ai = AiService::new();
    let sandbox = SandBox::new();

    let mut problem = None;
    for attempt in 0..3 {
        match ai.generate_problem("rust", "medium").await {
            Ok(p) => {
                if validate_problem(&sandbox, &p, "rust").await.is_ok() {
                    problem = Some(p);
                    break;
                }
            }
            Err(e) => tracing::warn!("AI attempt {} failed: {}", attempt, e),
        }
    }

    let problem = match problem {
        Some(p) => p,
        None => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate valid problem".to_string())),
    };

    db::create_problem(&pool, &problem, "rust", Difficulty::Medium)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Redirect::to("/problems"))
}

async fn complete_login(
    pool: &DbPool,
    session: &Session,
    user_id: Uuid,
) -> Result<(), String> {
    update_last_login(pool, user_id).await
        .map_err(|e| e.to_string())?;
    session.insert(SESSION_USER_ID, user_id.to_string()).await
        .map_err(|e| e.to_string())?;

    merge_guest_progress(pool, session, user_id).await;

    Ok(())
}

#[allow(unused)]
async fn merge_guest_progress(pool: &DbPool, session: &Session, user_id: Uuid) {
    let solved: Vec<String> = session
        .get("solved_problems").await.ok().flatten()
        .unwrap_or_default();

    for problem_id_str in solved {
        if let Ok(pid) = Uuid::parse_str(&problem_id_str) {
            let _ = sqlx::query(
                "UPDATE submissions SET user_id = $1
                 WHERE session_id = $2 AND problem_id = $3 AND user_id IS NULL"
            )
                .bind(user_id)
                .bind(format!("{:?}", session.id()))
                .bind(pid)
                .execute(pool)
                .await;
        }
    }

    let _ = session.remove::<Vec<String>>("solved_problems").await;
}

pub async fn test_generate_problem(
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if admin_token.is_empty() || provided != admin_token {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    let ai = AiService::new();

    for attempt in 0..3 {
        match ai.generate_problem("rust", "easy").await {
            Ok(problem) => {
                return Ok(Json(json!({
                    "success": true,
                    "problem": problem,
                    "attempt": attempt + 1
                })));
            }
            Err(e) => {
                tracing::warn!("Attempt {} failed: {}", attempt + 1, e);
            }
        }
    }

    Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate problem after 3 attempts".to_string()))
}