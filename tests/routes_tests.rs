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
        get_register,
        get_login,
        get_profile,
        // get_problems
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

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/problems");
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
        assert!(std::str::from_utf8(&body).unwrap().contains("already exists"));
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore]
    async fn test_login_success(pool: PgPool) {
        let app = create_router(pool.clone()).await;
        app.oneshot(form_request("/register", REGISTER_BODY))
            .await
            .unwrap();

        let app = create_router(pool).await;
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
        assert!(std::str::from_utf8(&body).unwrap().contains("Invalid username/email or password"));
    }

    #[tokio::test]
    async fn test_get_register_returns_html_with_flash_message() {
        let result = get_register(Some("Test error message".to_string())).await;
        let html = result.0;
        assert!(html.contains("Test error message"));
        assert!(html.contains("Sign up"));
        assert!(html.contains("Username"));
        assert!(html.contains("Email"));
        assert!(html.contains("Password"));
        assert!(html.contains("Confirm password"));
        assert!(html.contains("/register"));
    }

    #[tokio::test]
    async fn test_get_register_returns_html_without_flash_message() {
        let result = get_register(None).await;
        let html = result.0;
        assert!(html.contains("Sign up"));
        assert!(html.contains("Username"));
        assert!(html.contains("Email"));
        assert!(html.contains("Password"));
        assert!(html.contains("Confirm password"));
        assert!(html.contains("/register"));
    }

    #[tokio::test]
    async fn test_get_login_returns_html_with_flash_message() {
        let result = get_login(Some("Invalid credentials".to_string())).await;
        let html = result.0;
        assert!(html.contains("Invalid credentials"));
        assert!(html.contains("Sign in"));
        assert!(html.contains("Username or Email"));
        assert!(html.contains("Password"));
        assert!(html.contains("/login"));
    }

    #[tokio::test]
    async fn test_get_login_returns_html_without_flash_message() {
        let result = get_login(None).await;
        let html = result.0;
        assert!(html.contains("Sign in"));
        assert!(html.contains("Username or Email"));
        assert!(html.contains("Password"));
        assert!(html.contains("/login"));
    }

    #[tokio::test]
    async fn test_get_profile_returns_html() {
        let result = get_profile(None).await;
        let html = result.0;
        assert!(html.contains("Profile"));
        assert!(html.contains("Problems"));
        assert!(html.contains("Sign in"));
    }

    #[tokio::test]
    async fn test_get_profile_works_with_flash_message() {
        let result = get_profile(Some("Welcome!".to_string())).await;
        let html = result.0;
        assert!(html.contains("Profile"));
    }

    // #[ignore]
    // #[tokio::test]
    // async fn test_get_problems_returns_html() {
    //     let result = get_problems(None).await;
    //     let html = result.0;
    //     assert!(html.contains("Problems"));
    //     assert!(html.contains("Sign in"));
    // }
    //
    // #[tokio::test]
    // async fn test_get_problems_works_with_flash_message() {
    //     let result = get_problems(Some("New problem added".to_string())).await;
    //     let html = result.0;
    //     assert!(html.contains("Problems"));
    // }
}