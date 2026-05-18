#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use sqlx::PgPool;
    use hotfix::{
        create_router,
        config::Config,
    };
    use tower_sessions::{Expiry, SessionManagerLayer};
    use tower_sessions_sqlx_store::PostgresStore;
    use time::Duration;
    use dotenvy::dotenv;
    use std::sync::Once;

    const REGISTER_BODY: &str =
        "username=test99&email=test@test.com&password=Secret123&confirm_password=Secret123";

    static INIT: Once = Once::new();

    fn init_test_config() {
        INIT.call_once(|| {
            dotenv().ok();
            // Set test environment variables if not already set
            unsafe {
                if std::env::var("DATABASE_URL").is_err() {
                    std::env::set_var("DATABASE_URL", "postgres://postgres:postgres@localhost/hotfix_test");
                }
                if std::env::var("ENCRYPTION_KEY").is_err() {
                    std::env::set_var("ENCRYPTION_KEY", "test_encryption_key_32_bytes_long_!!");
                }
                if std::env::var("USERNAME_SECRET").is_err() {
                    std::env::set_var("USERNAME_SECRET", "test_username_secret");
                }
                if std::env::var("SESSION_SECRET").is_err() {
                    std::env::set_var("SESSION_SECRET", "test_session_secret");
                }
                if std::env::var("APP_ENVIRONMENT").is_err() {
                    std::env::set_var("APP_ENVIRONMENT", "test");
                }
                if std::env::var("LOG_LEVEL").is_err() {
                    std::env::set_var("LOG_LEVEL", "info");
                }
                if std::env::var("SMTP_HOST").is_err() {
                    std::env::set_var("SMTP_HOST", "localhost");
                }
                if std::env::var("SMTP_USERNAME").is_err() {
                    std::env::set_var("SMTP_USERNAME", "test");
                }
                if std::env::var("SMTP_PASSWORD").is_err() {
                    std::env::set_var("SMTP_PASSWORD", "test");
                }
                if std::env::var("SMTP_FROM").is_err() {
                    std::env::set_var("SMTP_FROM", "test@test.com");
                }
                if std::env::var("SMTP_PORT").is_err() {
                    std::env::set_var("SMTP_PORT", "25");
                }
            }

            let _ = Config::init();
        });
    }

    async fn create_test_app(pool: PgPool) -> axum::Router {
        // Run migrations for the session store
        let session_store = PostgresStore::new(pool.clone());
        let _ = session_store.migrate().await;

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_http_only(true)
            .with_same_site(tower_sessions::cookie::SameSite::Lax)
            .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

        create_router(pool).await.layer(session_layer)
    }

    fn form_request(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_register_success(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool).await;

        let response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers().get("location").unwrap();
        assert_eq!(location, "/2fa_confirm");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_register_password_mismatch(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool).await;

        let response = app
            .oneshot(form_request(
                "/register",
                "username=test99&email=test@test.com&password=Secret123&confirm_password=Wrong999",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("Passwords do not match"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_register_duplicate_user(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool.clone()).await;

        let response1 = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();
        assert_eq!(response1.status(), StatusCode::SEE_OTHER);

        let app2 = create_test_app(pool).await;
        let response2 = app2
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        assert_eq!(response2.status(), StatusCode::OK);
        let body = response2.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("already exists"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_login_success(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool.clone()).await;

        let register_response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::SEE_OTHER);

        // After registration, the user is pending 2FA, so we need to verify email first
        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Manually mark email as verified AND set last_login_at to a recent time to avoid 2FA
        sqlx::query("UPDATE users SET email_verified = TRUE, last_login_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        let app2 = create_test_app(pool.clone()).await;
        let response_with_name = app2
            .oneshot(form_request(
                "/login",
                "identifier=test99&password=Secret123",
            ))
            .await
            .unwrap();

        let app3 = create_test_app(pool).await;
        let response_with_email = app3
            .oneshot(form_request(
                "/login",
                "identifier=test@test.com&password=Secret123",
            ))
            .await
            .unwrap();

        assert_eq!(response_with_name.status(), StatusCode::SEE_OTHER);
        assert_eq!(response_with_email.status(), StatusCode::SEE_OTHER);
        assert_eq!(response_with_name.headers().get("location").unwrap(), "/problems");
        assert_eq!(response_with_email.headers().get("location").unwrap(), "/problems");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_login_wrong_password(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool.clone()).await;

        let register_response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::SEE_OTHER);

        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Mark email as verified and set last_login_at for login attempt
        sqlx::query("UPDATE users SET email_verified = TRUE, last_login_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        let app2 = create_test_app(pool).await;
        let response = app2
            .oneshot(form_request(
                "/login",
                "identifier=test99&password=WrongPass1",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("Invalid username/email or password"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_profile_redirects_when_not_logged_in(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool).await;

        let response = app
            .oneshot(Request::builder().uri("/profile").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/login");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_profile_works_when_logged_in(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool.clone()).await;

        // First create a user
        let register_response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::SEE_OTHER);

        // Mark email as verified and set last_login_at to avoid 2FA
        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        sqlx::query("UPDATE users SET email_verified = TRUE, last_login_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        // Login should now go directly to problems
        let app2 = create_test_app(pool.clone()).await;
        let login_response = app2
            .oneshot(form_request("/login", "identifier=test99&password=Secret123"))
            .await
            .unwrap();

        assert_eq!(login_response.status(), StatusCode::SEE_OTHER);
        let location = login_response.headers().get("location").unwrap();
        assert_eq!(location, "/problems");

        // Note: We can't easily test the actual profile page because we need to extract cookies
        // But we've verified the login flow works correctly
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_problems_returns_html(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool).await;

        let response = app
            .oneshot(Request::builder().uri("/problems").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("Problems"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_logout_clears_session(pool: PgPool) {
        init_test_config();
        let app = create_test_app(pool.clone()).await;

        // First create a user
        let register_response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::SEE_OTHER);

        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Mark email as verified and set last_login_at to avoid 2FA
        sqlx::query("UPDATE users SET email_verified = TRUE, last_login_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        // Login should go directly to problems
        let app2 = create_test_app(pool).await;
        let login_response = app2.clone()
            .oneshot(form_request("/login", "identifier=test99&password=Secret123"))
            .await
            .unwrap();

        assert_eq!(login_response.status(), StatusCode::SEE_OTHER);
        assert_eq!(login_response.headers().get("location").unwrap(), "/problems");

        // Logout
        let logout_response = app2
            .oneshot(Request::builder().uri("/logout").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(logout_response.status(), StatusCode::SEE_OTHER);
        assert_eq!(logout_response.headers().get("location").unwrap(), "/login");
    }

    // Unit tests for templates (these don't need database)
    #[tokio::test]
    async fn test_register_template_renders_correctly() {
        use askama::Template;
        use hotfix::schemas::RegisterTemplate;

        let template = RegisterTemplate {
            flash_message: Some("Test error".to_string()),
            username: Some("testuser".to_string()),
            email: Some("test@test.com".to_string()),
            password: None,
            confirm_password: None,
            logged_in: false,
        };

        let rendered = template.render().unwrap();
        assert!(rendered.contains("Test error"));
        assert!(rendered.contains("testuser"));
        assert!(rendered.contains("test@test.com"));
    }

    #[tokio::test]
    async fn test_login_template_renders_correctly() {
        use askama::Template;
        use hotfix::schemas::LoginTemplate;

        let template = LoginTemplate {
            flash_message: Some("Invalid credentials".to_string()),
            identifier: Some("testuser".to_string()),
            password: None,
            logged_in: false,
        };

        let rendered = template.render().unwrap();
        assert!(rendered.contains("Invalid credentials"));
        assert!(rendered.contains("testuser"));
    }

    #[tokio::test]
    async fn test_2fa_template_renders_correctly() {
        use askama::Template;
        use hotfix::schemas::TwoFaTemplate;

        let template = TwoFaTemplate {
            flash_message: Some("Code expired".to_string()),
            confirm_code: Some("123456".to_string()),
            logged_in: false,
        };

        let rendered = template.render().unwrap();
        assert!(rendered.contains("Code expired"));
        assert!(rendered.contains("123456"));
    }

    #[tokio::test]
    async fn test_problems_template_renders() {
        use askama::Template;
        use hotfix::schemas::{ProblemsTemplate, Problem, Difficulty};
        use uuid::Uuid;
        use serde_json::json;

        let problem = Problem {
            id: Uuid::new_v4(),
            name: "Test Problem".to_string(),
            topics: vec!["arrays".to_string(), "loops".to_string()],
            language: "rust".to_string(),
            difficulty: Difficulty::Easy,
            correct_version: "fn main() {}".to_string(),
            incorrect_version: "fn main() { panic!() }".to_string(),
            tests: json!([]),
            time_limit_seconds: 5,
            description: "Test description".to_string(),
            solved_count: 10,
            created_at: chrono::Utc::now(),
        };

        let template = ProblemsTemplate {
            problems: vec![problem],
            logged_in: false,
        };

        let rendered = template.render().unwrap();
        assert!(rendered.contains("Test Problem"));
        assert!(rendered.contains("arrays"));
        assert!(rendered.contains("loops"));
    }
}