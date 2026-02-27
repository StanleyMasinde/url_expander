use std::sync::Arc;

use axum::{
    Extension,
    extract::Request,
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::auth::{error::AuthError, service::AuthService};

#[derive(Clone)]
pub struct AuthContext {
    pub user_id: u64,
}

pub async fn require_auth(
    Extension(auth_service): Extension<Arc<AuthService>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(header_value) = request.headers().get(header::AUTHORIZATION) else {
        return AuthError::Unauthorized.into_response();
    };

    let Ok(header_str) = header_value.to_str() else {
        return AuthError::Unauthorized.into_response();
    };

    let Some(token) = header_str.strip_prefix("Bearer ") else {
        return AuthError::Unauthorized.into_response();
    };

    let user_id = match auth_service.validate_access_token(token) {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };

    request.extensions_mut().insert(AuthContext { user_id });
    next.run(request).await
}

pub async fn require_https_in_production(
    Extension(auth_service): Extension<Arc<AuthService>>,
    request: Request,
    next: Next,
) -> Response {
    if auth_service.is_https_required() {
        let forwarded_proto = request
            .headers()
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("http");

        if forwarded_proto != "https" {
            return AuthError::HttpsRequired.into_response();
        }
    }

    next.run(request).await
}
