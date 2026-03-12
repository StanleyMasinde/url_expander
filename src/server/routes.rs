use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    extract::{Query, State},
    middleware,
    response::IntoResponse,
    routing::get,
};
use dashmap::DashMap;
use log::error;
use reqwest::{Method, StatusCode};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{
    expander, proxy, request,
    server::{AppState, middleware::cache},
    types::{Cache, DISK_CACHE, Storage, Transport},
    utils::{job_runner::job_runner, reqwest_error::handle_reqwest_error},
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
    let state = AppState {
        client,
        memory_cache: Cache::new(),
    };
    let limiter = RateLimiter {
        buckets: Arc::new(DashMap::new()),
    };

    tokio::spawn(job_runner(state.memory_cache.clone()));
    Router::new()
        .route("/", get(index_handler))
        .route("/proxy", get(proxy_url))
        .layer(middleware::from_fn_with_state(limiter.clone(), rate_limit))
        .layer(middleware::from_fn_with_state(
            state.memory_cache.clone(),
            cache::cache,
        ))
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
    let cache = state.memory_cache;

    if let Some(url) = params.get("url") {
        match expander::expand_url(url, client).await {
            Ok(expanded_url) => {
                if let Err(e) = cache.set(url, expanded_url.to_string()).await {
                    error!("Failed to expand {}: {}", url, e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "An error occoured while trying to expand the url. Our team has been notified".into());
                };
                (StatusCode::OK, expanded_url)
            }
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
            Ok(html) => {
                let app_cache = DISK_CACHE.get_or_init(|| Cache::new().with_storage(Storage::Disk));
                if let Err(e) = app_cache.set(url, html.to_string()).await {
                    error!("Failed to proxy {}: {}", url, e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "An error occoured while trying to fetch the preview for {}, our team has been notified.".into());
                };
                (StatusCode::OK, html.to_string())
            }
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}
