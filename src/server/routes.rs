use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    extract::{Query, State},
    middleware,
    response::IntoResponse,
    routing::get,
};
use dashmap::DashMap;
use reqwest::{Method, StatusCode};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{
    expander, proxy, request, server::AppState, utils::reqwest_error::handle_reqwest_error,
};
use crate::{server::middleware::rate_limit::rate_limit, types::RateLimiter};

pub fn routes() -> Router {
    Router::new()
        .nest("/api", api_routes())
        .merge(index_routes())
}

fn index_routes() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(AllowOrigin::mirror_request());
    let client = request::create_reqwest();
    let state = AppState { client };
    let limiter = RateLimiter {
        buckets: Arc::new(DashMap::new()),
    };
    Router::new()
        .route("/", get(index_handler))
        .route("/proxy", get(proxy_url))
        .layer(middleware::from_fn_with_state(limiter.clone(), rate_limit))
        .with_state(state)
        .with_state(limiter)
        .layer(cors)
}

fn api_routes() -> Router {
    Router::new().route("/health", get(health_handler))
}

///
/// Methods here
///
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK".to_string())
}

async fn index_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;

    if let Some(url) = params.get("url") {
        match expander::expand_url(url, client).await {
            Ok(url) => (StatusCode::OK, url),
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}

async fn proxy_url(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;

    if let Some(url) = params.get("url") {
        match proxy::return_preview_html(url, client).await {
            Ok(html) => (StatusCode::OK, html.to_string()),
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}
