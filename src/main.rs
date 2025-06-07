use rand::{prelude::IndexedRandom, rng};
use serde::Serialize;
mod expander;
use std::{collections::HashMap, env::args, net::SocketAddr};

use axum::{Router, extract::Query, http::HeaderValue, response::IntoResponse, routing::get};
use hyper::{HeaderMap, StatusCode, header};

#[tokio::main]
async fn main() {
    let args: Vec<String> = args().collect();
    let binding = String::from("3000");
    let port = args.get(1).unwrap_or(&binding);

    let port_number = port
        .parse::<u16>()
        .map_err(|e| format!("The port has to be a number: {:?}", e))
        .unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port_number));

    // build our application with a single route
    let app = Router::new().route("/", get(expand_url));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize)]
struct UrlParams {
    url: String,
}

async fn expand_url(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let url_param = params.get("url").map_or("https://example.com", |v| v);

    // We can visit the upstream URL
    let parsed_url = url_param.parse::<hyper::Uri>();

    if parsed_url.is_err() {
        return (
            StatusCode::BAD_REQUEST,
            String::from("Failed to parse the URL please check it"),
        );
    }

    let expanded_url_response = match follow_endpoint(parsed_url.unwrap().to_string()).await {
        Ok(url) => url,
        Err(err) => format!("{}", err),
    };

    let mut status = StatusCode::OK;

    if expanded_url_response.contains("Failed") {
        status = StatusCode::BAD_GATEWAY
    }

    (status, expanded_url_response)
}

/// Follows redirects and returns the final resolved URL as a `String`.
///
/// For example, `https://youtu.be/...` will return `https://www.youtube.com/...`
///
/// # Errors
/// Returns an error if the request fails or the URL cannot be resolved.
async fn follow_endpoint(endpoint: String) -> Result<String, Box<dyn std::error::Error>> {
    let headers = build_headers(&endpoint);

    let client = reqwest::Client::new();

    let resp = client
        .head(&endpoint)
        .headers(headers)
        .send()
        .await
        .map_err(|_er| format!("Failed to make Request to: {}", endpoint))?;

    Ok(resp.url().to_string())
}

/// .
/// Randomize user agent
/// # Panics
///
/// Panics if .
fn randomize_user_agent(endpoint: &str) -> String {
    let user_agents = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edg/113.0.1774.50 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_4) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/113.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 12_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/113.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_2_3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.82 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Brave/113.0.5672.126 Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Vivaldi/5.6.2867.58 Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.3 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:93.0) Gecko/20100101 Firefox/93.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0.4606.61 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Opera/80.0.4170.63 Safari/537.36",
    ];
    if endpoint.contains("facebook.com") || endpoint.contains("instagram.com") {
        "curl/8.7.1".to_string()
    } else {
        let mut rng = rng();
        let random_user_agent = user_agents.choose(&mut rng).unwrap();
        random_user_agent.to_string()
    }
}

/// .
/// Return the headers to be used
/// # Panics
///
/// Panics if .
fn build_headers(endpoint: &str) -> hyper::HeaderMap {
    let user_agent = randomize_user_agent(endpoint);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_str(&user_agent).expect("Invalid user agent"),
    );
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        ),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.5"),
    );
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));

    headers.insert(
        header::REFERER,
        HeaderValue::from_str("https://www.google.com/").unwrap(),
    );

    headers
}
