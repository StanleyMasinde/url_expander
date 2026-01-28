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
    expander, proxy, request,
    server::{AppState, middleware::cache},
    utils::{
        cache::{Cache, Storage, Transport},
        reqwest_error::handle_reqwest_error,
    },
};
use crate::{server::middleware::rate_limit::rate_limit, types::RateLimiter};

pub fn routes() -> Router {
    Router::new()
        .nest("/api", api_routes())
        .merge(index_routes())
}

/// Builds the top-level Router for the application, wiring the index and proxy routes, CORS,
/// in-memory cache, and rate-limiting middleware into the shared application state.
///
/// The router mounts:
/// - GET "/" -> index_handler
/// - GET "/proxy" -> proxy_url
///
/// # Examples
///
/// ```
/// let router = index_routes();
/// // `router` can be served with `axum::Server::bind(...).serve(router.into_make_service())`
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

/// Handles index requests by expanding a provided `url` query parameter, caching the expanded URL, and returning the expansion or an appropriate error response.
///
/// On success, returns an HTTP 200 response containing the expanded URL and stores the expanded value in the in-memory cache. If the `url` query parameter is missing, returns HTTP 400 with the message `"URL parameter missing"`. If URL expansion fails, returns an error response reflecting that failure.
///
/// # Examples
///
/// ```no_run
/// use std::collections::HashMap;
/// use axum::extract::{State, Query};
///
/// // Construct minimal inputs (placeholders shown; real AppState and client required).
/// let state = /* State<AppState> */ unimplemented!();
/// let mut params = HashMap::new();
/// params.insert("url".to_string(), "http://example.com".to_string());
///
/// // Call the handler (async context required).
/// // let response = index_handler(State(state), Query(params)).await;
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
                cache.set(url, expanded_url.to_string()).unwrap();
                (StatusCode::OK, expanded_url)
            }
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}

/// Returns preview HTML for the `url` query parameter and caches it on disk.
///
/// If the `url` query parameter is present, retrieves a preview HTML for that URL, stores the HTML in a disk-backed cache keyed by the original URL, and responds with HTTP 200 and the HTML body. If the `url` parameter is missing, responds with HTTP 400 and the message `"URL parameter missing"`. Network errors are translated into appropriate HTTP responses via `handle_reqwest_error`.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use axum::extract::{Query, State};
/// use http::StatusCode;
///
/// #[tokio::test]
/// async fn proxy_url_missing_param_returns_bad_request() {
///     // Construct minimal state; the client is not used when the `url` param is absent.
///     let state = crate::server::AppState {
///         client: crate::request::create_reqwest(),
///         memory_cache: crate::cache::Cache::new(),
///     };
///     let params: HashMap<String, String> = HashMap::new();
///     let response = crate::server::proxy_url(State(state), Query(params)).await.into_response();
///     assert_eq!(response.status(), StatusCode::BAD_REQUEST);
/// }
/// ```
async fn proxy_url(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;

    if let Some(url) = params.get("url") {
        match proxy::return_preview_html(url, client).await {
            Ok(html) => {
                let app_cache = Cache::new().with_storage(Storage::Disk);
                app_cache.set(url, html.to_string()).unwrap();
                (StatusCode::OK, html.to_string())
            }
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}