use crate::types::{Cache, DISK_CACHE, Storage, Transport};
use axum::{
    extract::{Query, Request, State},
    middleware::Next,
    response::Response,
};
use axum_macros::debug_middleware;
use log::debug;
use reqwest::Method;
use std::collections::HashMap;

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
        let cache_entry = memory_cache.get(url).unwrap_or_default();
        if let Some(entry) = cache_entry {
            return Response::builder()
                .body(entry.into())
                .unwrap_or_else(|_| Response::new(String::new().into()));
        }
    };
    if request_method == Method::GET && request_path == "/proxy" {
        debug!("Looking for cache in disk");
        let cache = DISK_CACHE.get_or_init(|| Cache::new().with_storage(Storage::Disk));
        let cache_entry = cache.get(url).unwrap_or_default();
        if let Some(entry) = cache_entry {
            return Response::builder()
                .body(entry.into())
                .unwrap_or_else(|_| Response::new(String::new().into()));
        }
    }
    debug!("No cache hit for {}", url);
    next.run(request).await
}
