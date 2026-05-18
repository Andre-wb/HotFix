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
    };

    const REGISTER_BODY: &str =
        "username=test99&email=test@test.com&password=Secret123&confirm_password=Secret123";

    fn form_request(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore]
    async fn test_register_success(pool: PgPool) {
        let app = create_router(pool).await;

        let response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        // Should redirect to 2FA verification
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/2fa_confirm");
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore]
    async fn test_register_password_mismatch(pool: PgPool) {
        let app = create_router(pool).await;

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
    #[ignore]
    async fn test_register_duplicate_user(pool: PgPool) {
        let app = create_router(pool.clone()).await;
        app.oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        let app = create_router(pool).await;
        let response = app
            .oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("already exists"));
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore]
    async fn test_login_success(pool: PgPool) {
        let app = create_router(pool.clone()).await;
        app.oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        let app = create_router(pool.clone()).await;

        // After registration, the user is pending 2FA, so we need to verify email first
        // For testing purposes, we'll mark email as verified in the database
        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Manually mark email as verified for testing
        sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        let response_with_name = app.clone()
            .oneshot(form_request(
                "/login",
                "identifier=test99&password=Secret123",
            ))
            .await
            .unwrap();

        let response_with_email = app
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
    #[ignore]
    async fn test_login_wrong_password(pool: PgPool) {
        let app = create_router(pool.clone()).await;
        app.oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        let app = create_router(pool).await;
        let response = app
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
        let app = create_router(pool).await;

        let response = app
            .oneshot(Request::builder().uri("/profile").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should redirect to login when not authenticated
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/login");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_profile_works_when_logged_in(pool: PgPool) {
        // First create a user
        let app = create_router(pool.clone()).await;
        app.clone().oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        // Mark email as verified
        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        // Login to get session
        let login_response = app.clone()
            .oneshot(form_request("/login", "identifier=test99&password=Secret123"))
            .await
            .unwrap();

        // Extract session cookie from login response
        let cookies = login_response.headers().get("set-cookie");
        assert!(cookies.is_some(), "Login should set a session cookie");

        // Create a new request to profile with the session cookie
        let profile_request = Request::builder()
            .uri("/profile")
            .header("cookie", cookies.unwrap().to_str().unwrap())
            .body(Body::empty())
            .unwrap();

        let profile_response = app
            .oneshot(profile_request)
            .await
            .unwrap();

        assert_eq!(profile_response.status(), StatusCode::OK);
        let body = profile_response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("test99"));
        assert!(body_str.contains("Profile"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_problems_returns_html(pool: PgPool) {
        let app = create_router(pool).await;

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
        // First create and login a user
        let app = create_router(pool.clone()).await;
        app.clone().oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        let user = sqlx::query_as::<_, hotfix::User>(
            "SELECT * FROM users WHERE username = $1"
        )
            .bind("test99")
            .fetch_one(&pool)
            .await
            .unwrap();

        sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        // Login
        let login_response = app.clone()
            .oneshot(form_request("/login", "identifier=test99&password=Secret123"))
            .await
            .unwrap();

        let cookies = login_response.headers().get("set-cookie").unwrap();

        // Access profile while logged in
        let profile_request = Request::builder()
            .uri("/profile")
            .header("cookie", cookies.to_str().unwrap())
            .body(Body::empty())
            .unwrap();

        let profile_response = app
            .clone()
            .oneshot(profile_request)
            .await
            .unwrap();

        assert_eq!(profile_response.status(), StatusCode::OK);

        // Logout
        let logout_request = Request::builder()
            .uri("/logout")
            .header("cookie", cookies.to_str().unwrap())
            .body(Body::empty())
            .unwrap();

        let logout_response = app
            .clone()
            .oneshot(logout_request)
            .await
            .unwrap();

        assert_eq!(logout_response.status(), StatusCode::SEE_OTHER);
        assert_eq!(logout_response.headers().get("location").unwrap(), "/login");

        // Try to access profile after logout (should redirect)
        let profile_after_logout = Request::builder()
            .uri("/profile")
            .header("cookie", cookies.to_str().unwrap())
            .body(Body::empty())
            .unwrap();

        let final_response = app
            .oneshot(profile_after_logout)
            .await
            .unwrap();

        assert_eq!(final_response.status(), StatusCode::SEE_OTHER);
        assert_eq!(final_response.headers().get("location").unwrap(), "/login");
    }

    // Unit tests for the route handlers without creating sessions
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
}