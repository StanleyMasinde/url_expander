use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    http::HeaderValue,
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

    let map = limiter.buckets;
    let now = Instant::now();

    let mut bucket = map.entry(fingerprint).or_insert(Bucket {
        tokens: CAPACITY,
        last_refill: now,
    });

    let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
    bucket.tokens = (bucket.tokens.floor() + elapsed * REFILL_RATE)
        .min(CAPACITY)
        .floor();
    bucket.last_refill = now;

    if bucket.tokens < 1.0 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let limit_reset = now.as_secs_f64() + (1.0 - bucket.tokens.floor()) / REFILL_RATE;
        let retry_after = limit_reset - now.as_secs_f64();
        return Response::builder()
            .header("Retry-After", retry_after.floor().to_string())
            .header("X-RateLimit-Limit", CAPACITY.to_string())
            .header("X-RateLimit-Remaining", bucket.tokens.to_string())
            .header("X-RateLimit-Reset", limit_reset.floor().to_string())
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body("Rate limit exceeded".into())
            .unwrap();
    }

    bucket.tokens -= 1.0;
    let mut response = next.run(request).await;

    response
        .headers_mut()
        .insert("X-RateLimit-Limit", CAPACITY.to_string().parse().unwrap());
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        bucket.tokens.to_string().parse().unwrap(),
    );
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    response.headers_mut().insert(
        "X-RateLimit-Reset",
        HeaderValue::from_str(&current_time.as_secs_f64().floor().to_string()).unwrap(),
    );

    response
}
