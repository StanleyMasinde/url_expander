use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    http::{HeaderMap, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;

use crate::auth::{
    error::AuthError,
    models::{LoginRequest, LogoutRequest, MeResponse, RefreshRequest, RegisterRequest},
    service::AuthService,
};

use super::middleware::auth::{AuthContext, require_auth, require_https_in_production};

pub fn auth_routes(auth_service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route(
            "/auth/logout",
            post(logout)
                .route_layer(middleware::from_fn(require_auth))
                .route_layer(middleware::from_fn(require_https_in_production)),
        )
        .route(
            "/auth/me",
            get(me)
                .route_layer(middleware::from_fn(require_auth))
                .route_layer(middleware::from_fn(require_https_in_production)),
        )
        .layer(Extension(auth_service))
}

pub fn auth_disabled_routes() -> Router {
    Router::new()
        .route("/auth/register", post(auth_not_configured))
        .route("/auth/login", post(auth_not_configured))
        .route("/auth/refresh", post(auth_not_configured))
        .route("/auth/logout", post(auth_not_configured))
        .route("/auth/me", get(auth_not_configured))
}

async fn register(
    Extension(auth_service): Extension<Arc<AuthService>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let tokens = auth_service
        .register(&payload.name, &payload.email, &payload.password)
        .await?;

    Ok((StatusCode::CREATED, Json(tokens)))
}

async fn login(
    Extension(auth_service): Extension<Arc<AuthService>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let ip = resolve_ip_address(&headers);

    let tokens = auth_service
        .login(&payload.email, &payload.password, &ip)
        .await?;

    Ok((StatusCode::OK, Json(tokens)))
}

async fn refresh(
    Extension(auth_service): Extension<Arc<AuthService>>,
    Json(payload): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let tokens = auth_service.refresh(&payload.refresh_token).await?;
    Ok((StatusCode::OK, Json(tokens)))
}

async fn logout(
    Extension(auth_service): Extension<Arc<AuthService>>,
    Extension(_auth): Extension<AuthContext>,
    Json(payload): Json<LogoutRequest>,
) -> Result<impl IntoResponse, AuthError> {
    auth_service.logout(&payload.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(
    Extension(auth_service): Extension<Arc<AuthService>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse, AuthError> {
    let (user_id, email, name) = auth_service.user_profile(auth.user_id).await?;
    Ok((
        StatusCode::OK,
        Json(MeResponse {
            user_id,
            email,
            name,
        }),
    ))
}

fn resolve_ip_address(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn auth_not_configured() -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "message": "Authentication service unavailable" })),
    )
}
