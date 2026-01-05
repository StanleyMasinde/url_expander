use std::time::Instant;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_macros::debug_middleware;
use reqwest::StatusCode;

use crate::types::{Bucket, RateLimiter};
use crate::utils::fingerprint::generate_fingerprint;

const CAPACITY: f64 = 10.0; // max requests
const REFILL_RATE: f64 = 1.0; // tokens per second

#[debug_middleware]
pub async fn rate_limit(
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let fingerprint = generate_fingerprint(&request);

    // I have created this inner scope to make sure that the lock
    // in in the map is dropped.
    // For some reason, manually calling drop before the last await does not work
    {
        let map = limiter.buckets;
        let now = Instant::now();

        let mut bucket = map.entry(fingerprint).or_insert(Bucket {
            tokens: CAPACITY,
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * REFILL_RATE).min(CAPACITY);
        bucket.last_refill = now;

        if bucket.tokens < 1.0 {
            return Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body("Rate limit exceeded".into())
                .unwrap();
        }

        bucket.tokens -= 1.0;
    }
    next.run(request).await
}
