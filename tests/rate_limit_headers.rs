use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use link_expander::server::routes::routes;
use tower::util::ServiceExt;

fn build_request() -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/?missing=url")
        .header("user-agent", "integration-test-agent")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn successful_request_includes_rate_limit_headers() {
    let app = routes();

    let response = app.oneshot(build_request()).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let headers = response.headers();
    assert_eq!(headers.get("X-RateLimit-Limit").unwrap(), "10");
    assert_eq!(headers.get("X-RateLimit-Remaining").unwrap(), "9");

    let reset = headers
        .get("X-RateLimit-Reset")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(reset > 0);
}

#[tokio::test]
async fn rate_limited_response_includes_retry_after_and_rate_limit_headers() {
    let app = routes();

    let mut limited_response = None;

    for _ in 0..30 {
        let response = app.clone().oneshot(build_request()).await.unwrap();
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            limited_response = Some(response);
            break;
        }
    }

    let response = limited_response.expect("expected to receive a 429 response");
    let headers = response.headers();

    assert_eq!(headers.get("X-RateLimit-Limit").unwrap(), "10");
    assert_eq!(headers.get("X-RateLimit-Remaining").unwrap(), "0");

    let retry_after = headers
        .get("Retry-After")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(retry_after <= 1);

    let reset = headers
        .get("X-RateLimit-Reset")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(reset > 0);
}
