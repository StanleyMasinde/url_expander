use std::collections::HashMap;

use axum::{
    extract::{Query, Request},
    middleware::Next,
    response::Response,
};
use axum_macros::debug_middleware;
use reqwest::Method;

use crate::utils::cache::{Cache, Storage, Transport};

#[debug_middleware]
pub async fn cache(
    params: Query<HashMap<String, String>>,
    request: Request,
    next: Next,
) -> Response {
    let request_path = request.uri().path();
    let request_method = request.method();
    if request_method == Method::GET && request_path == "/" {
        let cache = Cache::new();
        let url_to_expand = params.get("url").unwrap();
        let cache_entry = cache.get(url_to_expand).unwrap();

        if let Some(entry) = cache_entry {
            return Response::builder().body(entry.into()).unwrap();
        }
    };

    if request_method == Method::GET && request_path == "/proxy" {
        let cache = Cache::new().with_storage(Storage::Disk);
        let url_to_proxy = params.get("url").unwrap();
        let cache_entry = cache.get(url_to_proxy).unwrap();

        if let Some(entry) = cache_entry {
            return Response::builder().body(entry.into()).unwrap();
        }
    }

    next.run(request).await
}
