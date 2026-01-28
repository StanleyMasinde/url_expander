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

/// Builds the index-level Router configured with CORS, an in-memory cache, a rate limiter, and the index and proxy handlers.
///
/// The Router exposes GET "/" (index_handler) and GET "/proxy" (proxy_url), applies a rate-limit middleware and an in-memory cache middleware,
/// initializes AppState (including a reqwest client and a memory Cache), spawns the background cache job runner, and registers the RateLimiter state.
///
/// # Examples
///
/// ```
/// let _router: axum::Router = crate::server::index_routes();
/// ```
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

/// Handles requests to the index route by expanding a provided `url` query parameter and caching the result.
///
/// If the `url` query parameter is present, expands it using the configured HTTP client and stores the mapping (original URL -> expanded URL) in the in-memory cache. On successful expansion and caching, responds with status 200 and the expanded URL. If expansion fails, the error is delegated to `handle_reqwest_error`. If storing into the in-memory cache fails, responds with status 500 and an error message. If the `url` query parameter is missing, responds with status 400 and the message "URL parameter missing".
///
/// # Examples
///
/// ```no_run
/// use axum::extract::{State, Query};
/// use std::collections::HashMap;
/// // Construct `State(AppState)` and `Query(HashMap<String, String>)` as in your application,
/// // then call `index_handler(State(app_state), Query(params)).await` to get the response.
///
/// // Example (conceptual):
/// // let mut params = HashMap::new();
/// // params.insert("url".to_string(), "https://example.com".to_string());
/// // let response = index_handler(State(app_state), Query(params)).await;
/// // assert_eq!(response.into_response().status(), StatusCode::OK);
/// ```
async fn index_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;
    let cache = state.memory_cache;

    if let Some(url) = params.get("url") {
        match expander::expand_url(url, client).await {
            Ok(expanded_url) => {
                if let Err(e) = cache.set(url, expanded_url.to_string()) {
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

/// Fetches preview HTML for the `url` query parameter, stores the HTML in the disk-backed cache, and returns the HTML.
///
/// If the `url` parameter is missing this returns `400` with `"URL parameter missing"`. If caching the fetched HTML fails this returns `500` with an error message. Upstream request errors are handled by the request error handler and translated into an appropriate response.
///
/// # Examples
///
/// ```no_run
/// // Request: GET /proxy?url=https%3A%2F%2Fexample.com
/// // Successful response: 200 and preview HTML body.
/// ```
async fn proxy_url(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;

    if let Some(url) = params.get("url") {
        match proxy::return_preview_html(url, client).await {
            Ok(html) => {
                let app_cache = DISK_CACHE.get_or_init(|| Cache::new().with_storage(Storage::Disk));
                if let Err(e) = app_cache.set(url, html.to_string()) {
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