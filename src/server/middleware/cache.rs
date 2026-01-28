use crate::utils::cache::{Cache, Storage, Transport};
use axum::{
    extract::{Query, Request, State},
    middleware::Next,
    response::Response,
};
use axum_macros::debug_middleware;
use log::debug;
use reqwest::Method;
use std::collections::HashMap;
use std::sync::OnceLock;

static DISK_CACHE: OnceLock<Cache> = OnceLock::new();

/// Middleware that serves cached responses for GET requests based on a `url` query parameter.
///
/// When the query parameter `url` is missing the request is forwarded to the next handler.
/// - For GET requests to path `/`, attempts to return a cached response from the provided in-memory cache.
/// - For GET requests to path `/proxy`, attempts to return a cached response from a disk-backed cache (lazily initialized).
/// If a cache entry is found its body is returned as the response; on cache miss the request is forwarded to `next`.
///
/// # Returns
///
/// A `Response` containing the cached body when a cache hit occurs, or the response produced by calling `next.run(request).await` when there is no hit.
///
/// # Examples
///
/// ```
/// // Illustrative example (types and values simplified)
/// # use std::collections::HashMap;
/// # use axum::extract::State;
/// # use axum::extract::Query;
/// # use axum::http::Request;
/// # use axum::response::Response;
/// # async fn example(cache: crate::cache::Cache, next: axum::middleware::Next) {
/// let mut params = HashMap::new();
/// params.insert("url".to_string(), "https://example.com".to_string());
/// let state = State(cache);
/// let query = Query(params);
/// let request = Request::builder().uri("/?url=https://example.com").body(()).unwrap();
/// let response: Response = crate::server::middleware::cache(state, query, request, next).await;
/// # drop(response);
/// # }
/// ```
#[debug_middleware]
pub async fn cache(
    State(memory_cache): State<Cache>,
    params: Query<HashMap<String, String>>,
    request: Request,
    next: Next,
) -> Response {
    let request_path = request.uri().path();
    let request_method = request.method();
    let url = match params.get("url") {
        Some(u) => u,
        None => return next.run(request).await,
    };

    if request_method == Method::GET && request_path == "/" {
        debug!("Looking for cache in memory");
        let cache_entry = match memory_cache.get(url) {
            Ok(ent) => ent,
            Err(_) => None,
        };
        if let Some(entry) = cache_entry {
            return Response::builder()
                .body(entry.into())
                .unwrap_or_else(|_| Response::new(String::new().into()));
        }
    };
    if request_method == Method::GET && request_path == "/proxy" {
        debug!("Looking for cache in disk");
        let cache = DISK_CACHE.get_or_init(|| Cache::new().with_storage(Storage::Disk));
        let cache_entry = match cache.get(url) {
            Ok(ent) => ent,
            Err(_) => None,
        };
        if let Some(entry) = cache_entry {
            return Response::builder()
                .body(entry.into())
                .unwrap_or_else(|_| Response::new(String::new().into()));
        }
    }
    debug!("No cache hit for {}", url);
    next.run(request).await
}