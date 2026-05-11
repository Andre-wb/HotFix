/// GET and POST routes

use axum::{
    response::{Html, Redirect},
    extract::{State, Path},
    Form,
};
use askama::Template;
use chrono::Utc;
use tower_sessions::Session;
use uuid::Uuid;
use crate::email;
use crate::db::{self, verify_password, hash_password, DbPool};
use crate::schemas::{
    LoginTemplate,
    ProblemsTemplate,
    ProfileTemplate,
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


pub async fn get_register(flash_message: Option<String>) -> Html<String> {
    Html(RegisterTemplate {
        flash_message,
        username: None,
        email: None,
        password: None,
        confirm_password: None,
    }.render().unwrap())
}

pub async fn get_login(flash_message: Option<String>) -> Html<String> {
    Html(LoginTemplate {
        flash_message,
        identifier: None,
        password: None,
    }.render().unwrap())
}

pub async fn get_profile(_flash_message: Option<String>) -> Html<String> {
    Html(ProfileTemplate.render().unwrap())
}

pub async fn get_problems(State(pool): State<DbPool>) -> Html<String> {
    let problems = db::get_problems(&pool).await.unwrap_or_default();
    Html(ProblemsTemplate { problems }.render().unwrap())
}

pub async fn get_problem(
    State(pool): State<DbPool>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, Redirect> {
    let problem = match db::get_problem(&pool, id).await {
        Ok(Some(p)) => p,
        _ => return Err(Redirect::to("/problems")),
    };

    Ok(Html(
        ProblemTemplate {
            problem,
            start_time: Utc::now().timestamp(),
        }
            .render()
            .unwrap(),
    ))
}

pub async fn get_2fa(session: Session) -> Result<Html<String>, Redirect> {
    if session.get::<String>(SESSION_PENDING_USER).await.ok().flatten().is_none() {
        return Err(Redirect::to("/login"));
    }
    Ok(Html(TwoFaTemplate {
        flash_message: None,
        confirm_code: None,
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
            }.render().unwrap();
            return Err(Html(error_html));
        },
        Err(error) => {
            tracing::error!("Database error: {}", error);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again".to_string()),
                identifier: Some(form.identifier.clone()),
                password: Some(form.password.clone()),
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
            }.render().unwrap();
            return Err(Html(error_html));
        }
        Err(error) => {
            tracing::error!("Password verification error: {}", error);
            let error_html = LoginTemplate {
                flash_message: Some("An error occurred. Please try again.".to_string()),
                identifier: Some(form.identifier.clone()),
                password: Some(form.password.clone()),
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
                    }.render().unwrap();
                    return Err(Html(error_html));
                }
            }

            if let Err(error) = complete_login(&pool, &session, user_id).await {
                tracing::error!("Failed to complete authentication: {}", error);
                let error_html = TwoFaTemplate {
                    flash_message: Some("Error completing authentication. Please try again".to_string()),
                    confirm_code: Some(form.confirm_code.clone()),
                }.render().unwrap();
                return Err(Html(error_html));
            }

            Ok(Redirect::to("/problems"))
        }
        Ok(CodeVerificationResult::Invalid) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Incorrect code".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::Expired) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Code expired. Please go back and request a new one".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::TooManyAttempts) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Too many failed attempts. Please request a new code".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
            }.render().unwrap()))
        }
        Ok(CodeVerificationResult::AlreadyUsed) => {
            Err(Html(TwoFaTemplate {
                flash_message: Some("Code already used".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
            }.render().unwrap()))
        }
        Err(error) => {
            tracing::error!("Failed to verify code: {}", error);
            Err(Html(TwoFaTemplate {
                flash_message: Some("An error occurred".to_string()),
                confirm_code: Some(form.confirm_code.clone()),
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

    // Check time limit
    let elapsed = Utc::now().timestamp() - form.start_time;
    if elapsed > problem.time_limit_seconds as i64 {
        return Ok(Html("Time limit exceeded".to_string()));
    }

    let tests: Vec<TestCase> =
        serde_json::from_value(problem.tests).unwrap_or_default();

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
                if ok { passed += 1; }
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

    // Get user/session info
    let user_id = session
        .get::<String>(SESSION_USER_ID).await.ok().flatten()
        .and_then(|id| Uuid::parse_str(&id).ok());

    let session_id = format!("{:?}", session.id());

    // Save submission
    let _ = db::save_submission(
        &pool, problem_id, user_id, &session_id,
        &form.code, passed, tests.len() as i32, status,
    ).await;

    // Update stats
    if all_passed {
        let _ = db::increment_solved_count(&pool, problem_id).await;

        // Track in session for guests
        let mut solved: Vec<String> = session
            .get(SESSION_SOLVED).await.ok().flatten()
            .unwrap_or_default();

        if !solved.contains(&problem_id.to_string()) {
            solved.push(problem_id.to_string());
            let _ = session.insert(SESSION_SOLVED, solved).await;
        }

        // Update user stats if logged in
        if let Some(uid) = user_id {
            // Add to user's solved count, update tags, etc.
            let _ = sqlx::query(
                "UPDATE users SET problems_solved = problems_solved + 1 WHERE id = $1"
            )
                .bind(uid)
                .execute(&pool)
                .await;
        }
    }

    Ok(Html(
        ResultsTemplate {
            passed,
            total: tests.len() as i32,
            all_passed,
            results,
        }
            .render()
            .unwrap(),
    ))
}

// Admin endpoint to generate problems
pub async fn post_generate_problem(State(pool): State<DbPool>) -> Result<Redirect, String> {
    eprintln!("\n╔══════════════════════════════════════════════════════════════╗");
    eprintln!("║           PROBLEM GENERATION REQUEST RECEIVED                ║");
    eprintln!("╚══════════════════════════════════════════════════════════════╝");

    let ai = AiService::new();
    let sandbox = SandBox::new();

    eprintln!("\n📝 AI Service initialized");
    eprintln!("🔧 Sandbox initialized");

    let mut problem = None;
    let max_attempts = 3;

    for attempt in 0..max_attempts {
        eprintln!("\n┌────────────────────────────────────────────────────────────┐");
        eprintln!("│ ATTEMPT {}/{}                                              │", attempt + 1, max_attempts);
        eprintln!("└────────────────────────────────────────────────────────────┘");

        eprintln!("🤖 Requesting AI to generate problem (language: rust, difficulty: medium)...");

        match ai.generate_problem("rust", "medium").await {
            Ok(p) => {
                eprintln!("✓ AI generated problem successfully");
                eprintln!("  Title: {}", p.name);
                eprintln!("  Tests count: {}", p.tests.len());
                eprintln!("  Time limit: {} seconds", p.time_limit_seconds);
                eprintln!("  Correct version length: {} chars", p.correct_version.len());
                eprintln!("  Incorrect version length: {} chars", p.incorrect_version.len());

                eprintln!("\n🔍 Validating problem...");
                match validate_problem(&sandbox, &p, "rust").await {
                    Ok(()) => {
                        eprintln!("✅ Problem VALID!");
                        problem = Some(p);
                        break;
                    }
                    Err(e) => {
                        eprintln!("❌ Validation FAILED: {}", e);
                        eprintln!("   This problem will be discarded, retrying...");

                        // Log more details about the problem that failed
                        eprintln!("\n📋 Failed problem details:");
                        eprintln!("   Title: {}", p.name);
                        for (i, test) in p.tests.iter().enumerate() {
                            eprintln!("   Test {}: input={:?} expected={:?}",
                                      i, test.input, test.expected_output);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ AI generation FAILED: {}", e);
                eprintln!("   Error type: {}", std::any::type_name_of_val(&e));

                // Try to get more error context if possible
                if e.to_string().contains("timeout") {
                    eprintln!("   ⏰ This appears to be a timeout error");
                } else if e.to_string().contains("api") || e.to_string().contains("key") {
                    eprintln!("   🔑 This appears to be an API key or authentication error");
                } else if e.to_string().contains("network") {
                    eprintln!("   🌐 This appears to be a network connectivity error");
                }
            }
        }

        if attempt < max_attempts - 1 {
            eprintln!("\n⏳ Waiting 1 second before next attempt...");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    eprintln!("\n┌────────────────────────────────────────────────────────────┐");
    eprintln!("│ FINAL RESULT                                             │");
    eprintln!("└────────────────────────────────────────────────────────────┘");

    let problem = match problem {
        Some(p) => {
            eprintln!("✅ Problem generated successfully after validation!");
            eprintln!("   Title: {}", p.name);
            p
        }
        None => {
            eprintln!("❌ FAILED to generate a valid problem after {} attempts", max_attempts);
            eprintln!("\n🔧 TROUBLESHOOTING SUGGESTIONS:");
            eprintln!("   1. Check if AI service API key is properly configured");
            eprintln!("   2. Verify that the AI service endpoint is reachable");
            eprintln!("   3. Check if the AI model is returning valid code");
            eprintln!("   4. Verify sandbox execution environment is working");
            eprintln!("   5. Check if the AI is generating both correct and incorrect versions");
            eprintln!("   6. Verify test cases have proper input/output pairs");

            eprintln!("\n💡 To test individual components:");
            eprintln!("   - Test AI generation alone: cargo test test_ai_generation");
            eprintln!("   - Test sandbox alone: cargo test test_sandbox");
            eprintln!("   - Test validation alone with mock problem");

            return Err("Failed to generate valid problem after 3 attempts".to_string());
        }
    };

    eprintln!("\n💾 Saving problem to database...");
    match db::create_problem(&pool, &problem, "rust", Difficulty::Medium).await {
        Ok(_) => {
            eprintln!("✅ Problem saved to database successfully!");
            eprintln!("\n╔══════════════════════════════════════════════════════════════╗");
            eprintln!("║           PROBLEM GENERATION COMPLETE ✅                      ║");
            eprintln!("╚══════════════════════════════════════════════════════════════╝\n");
            Ok(Redirect::to("/problems"))
        }
        Err(e) => {
            eprintln!("❌ Failed to save problem to database: {}", e);
            Err(e.to_string())
        }
    }
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
    Ok(())
}

#[allow(unused)]
async fn merge_guest_progress(pool: &DbPool, session: &Session, user_id: Uuid) {
    let solved: Vec<String> = session
        .get("solved_problems").await.ok().flatten()
        .unwrap_or_default();

    for problem_id_str in solved {
        if let Ok(pid) = Uuid::parse_str(&problem_id_str) {
            // Update submissions to link to user
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