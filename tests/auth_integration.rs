
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use link_expander::{auth::build_auth_service, server::routes::routes_with_auth};
use serde_json::{Value, json};
use serial_test::serial;
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tower::util::ServiceExt;

fn test_database_url() -> Option<String> {
    std::env::var("AUTH_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

async fn setup_auth_app() -> Option<Router> {
    let database_url = test_database_url()?;

    // SAFETY: tests run serially and explicitly control process env for app configuration.
    unsafe {
        std::env::set_var("DATABASE_URL", database_url);
        std::env::set_var("APP_ENV", "development");
        std::env::set_var("JWT_HS256_SECRET", "integration-test-secret");
        std::env::set_var("JWT_ISSUER", "link_expander_tests");
        std::env::set_var("ACCESS_TOKEN_TTL_SECS", "1");
        std::env::set_var("REFRESH_TOKEN_TTL_SECS", "2592000");
    }

    let pool = connect_test_pool().await?;
    cleanup_tables(&pool).await.ok()?;

    let auth_service = build_auth_service().await.ok()?;
    Some(routes_with_auth(Some(auth_service)))
}

async fn connect_test_pool() -> Option<MySqlPool> {
    let database_url = test_database_url()?;

    MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .ok()
}

async fn cleanup_tables(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM users").execute(pool).await?;
    Ok(())
}

async fn json_request(
    app: &Router,
    method: &str,
    uri: &str,
    body: Value,
    auth_header: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    builder = builder.header("content-type", "application/json");

    if let Some(token) = auth_header {
        builder = builder.header("authorization", token);
    }

    let request = builder.body(Body::from(body.to_string())).unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
#[serial]
async fn register_flow_returns_validation_messages() {
    let Some(app) = setup_auth_app().await else {
        eprintln!("skipping test: AUTH_TEST_DATABASE_URL or DATABASE_URL not set");
        return;
    };

    let response = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "a", "email": "user@example.com", "password": "password123" }),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["field"], "name");
    assert_eq!(body["message"], "Name must be between 2 and 255 characters");

    let response = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "Valid Name", "email": "invalid", "password": "password123" }),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["field"], "email");
    assert_eq!(body["message"], "Please provide a valid email address");

    let response = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "Valid Name", "email": "user@example.com", "password": "short" }),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["field"], "password");
    assert_eq!(
        body["message"],
        "Password must be at least 8 characters long"
    );
}

#[tokio::test]
#[serial]
async fn login_flow_rejects_invalid_credentials() {
    let Some(app) = setup_auth_app().await else {
        eprintln!("skipping test: AUTH_TEST_DATABASE_URL or DATABASE_URL not set");
        return;
    };

    let register = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "Jane", "email": "jane@example.com", "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(register.status(), StatusCode::CREATED);

    let response = json_request(
        &app,
        "POST",
        "/auth/login",
        json!({ "email": "jane@example.com", "password": "wrong-password" }),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(response).await;
    assert_eq!(body["message"], "Email or password is incorrect");
}

#[tokio::test]
#[serial]
async fn refresh_flow_rotates_refresh_tokens() {
    let Some(app) = setup_auth_app().await else {
        eprintln!("skipping test: AUTH_TEST_DATABASE_URL or DATABASE_URL not set");
        return;
    };

    let register = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "John", "email": "john@example.com", "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(register.status(), StatusCode::CREATED);

    let tokens = response_json(register).await;
    let old_refresh = tokens["refresh_token"].as_str().unwrap().to_string();

    let refresh = json_request(
        &app,
        "POST",
        "/auth/refresh",
        json!({ "refresh_token": old_refresh }),
        None,
    )
    .await;
    assert_eq!(refresh.status(), StatusCode::OK);

    let refreshed_tokens = response_json(refresh).await;
    let new_refresh = refreshed_tokens["refresh_token"].as_str().unwrap();

    let old_token_retry = json_request(
        &app,
        "POST",
        "/auth/refresh",
        json!({ "refresh_token": tokens["refresh_token"] }),
        None,
    )
    .await;
    assert_eq!(old_token_retry.status(), StatusCode::UNAUTHORIZED);

    let new_token_use = json_request(
        &app,
        "POST",
        "/auth/refresh",
        json!({ "refresh_token": new_refresh }),
        None,
    )
    .await;
    assert_eq!(new_token_use.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn logout_revokes_refresh_token() {
    let Some(app) = setup_auth_app().await else {
        eprintln!("skipping test: AUTH_TEST_DATABASE_URL or DATABASE_URL not set");
        return;
    };

    let register = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "Maya", "email": "maya@example.com", "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(register.status(), StatusCode::CREATED);

    let tokens = response_json(register).await;
    let access_token = tokens["access_token"].as_str().unwrap();
    let refresh_token = tokens["refresh_token"].as_str().unwrap();

    let logout = json_request(
        &app,
        "POST",
        "/auth/logout",
        json!({ "refresh_token": refresh_token }),
        Some(&format!("Bearer {access_token}")),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let refresh = json_request(
        &app,
        "POST",
        "/auth/refresh",
        json!({ "refresh_token": refresh_token }),
        None,
    )
    .await;
    assert_eq!(refresh.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn login_rate_limit_enforced() {
    let Some(app) = setup_auth_app().await else {
        eprintln!("skipping test: AUTH_TEST_DATABASE_URL or DATABASE_URL not set");
        return;
    };

    for attempt in 1..=6 {
        let response = json_request(
            &app,
            "POST",
            "/auth/login",
            json!({ "email": "nobody@example.com", "password": "wrong-password" }),
            None,
        )
        .await;

        if attempt <= 5 {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        } else {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }
    }
}

#[tokio::test]
#[serial]
async fn end_to_end_auth_journey() {
    let Some(app) = setup_auth_app().await else {
        eprintln!("skipping test: AUTH_TEST_DATABASE_URL or DATABASE_URL not set");
        return;
    };

    let register = json_request(
        &app,
        "POST",
        "/auth/register",
        json!({ "name": "Alex", "email": "alex@example.com", "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(register.status(), StatusCode::CREATED);

    let mut tokens = response_json(register).await;

    let access = tokens["access_token"].as_str().unwrap();
    let me = json_request(
        &app,
        "GET",
        "/auth/me",
        json!({}),
        Some(&format!("Bearer {access}")),
    )
    .await;
    assert_eq!(me.status(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let expired = json_request(
        &app,
        "GET",
        "/auth/me",
        json!({}),
        Some(&format!("Bearer {access}")),
    )
    .await;
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);

    let refresh = json_request(
        &app,
        "POST",
        "/auth/refresh",
        json!({ "refresh_token": tokens["refresh_token"] }),
        None,
    )
    .await;
    assert_eq!(refresh.status(), StatusCode::OK);
    tokens = response_json(refresh).await;

    let refreshed_access = tokens["access_token"].as_str().unwrap();
    let me_after_refresh = json_request(
        &app,
        "GET",
        "/auth/me",
        json!({}),
        Some(&format!("Bearer {refreshed_access}")),
    )
    .await;
    assert_eq!(me_after_refresh.status(), StatusCode::OK);

    let logout = json_request(
        &app,
        "POST",
        "/auth/logout",
        json!({ "refresh_token": tokens["refresh_token"] }),
        Some(&format!("Bearer {refreshed_access}")),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let refresh_after_logout = json_request(
        &app,
        "POST",
        "/auth/refresh",
        json!({ "refresh_token": tokens["refresh_token"] }),
        None,
    )
    .await;
    assert_eq!(refresh_after_logout.status(), StatusCode::UNAUTHORIZED);
}
