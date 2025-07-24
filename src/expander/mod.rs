pub mod response;

use http_body_util::combinators::BoxBody;
use hyper::{
    Error, HeaderMap, Method, Request, Response, StatusCode, body::Bytes, header::HeaderValue,
};
use rand::{rng, seq::IndexedRandom};
use reqwest::{Client, header};
use response::build_response;

pub async fn handle_expansion(
    req: Request<hyper::body::Incoming>,
    client: Client,
) -> Result<Response<BoxBody<Bytes, Error>>, Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let query = req.uri().query().unwrap_or("");

            let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            let mut url_param = String::from("https://example.com");

            if let Some((_, value)) = params.iter().find(|(key, _)| key == "url") {
                let trimmed_val = value.trim();
                if value.starts_with("http") {
                    url_param = trimmed_val.to_string();
                } else {
                    url_param = String::from("https://") + trimmed_val
                }
            }

            // We can visit the upstream URL
            let parsed_url = url_param.parse::<hyper::Uri>();

            if parsed_url.is_err() {
                return build_response(
                    StatusCode::BAD_REQUEST,
                    String::from("Failed to parse the URL please check it"),
                );
            }

            let expanded_url_response =
                match follow_endpoint(parsed_url.unwrap().to_string(), client).await {
                    Ok(url) => url,
                    Err(err) => format!("{err}"),
                };

            let mut status = StatusCode::OK;

            if expanded_url_response.contains("Failed") {
                status = StatusCode::BAD_GATEWAY
            }

            build_response(status, expanded_url_response)
        }
        (&Method::GET, "/proxy") => {
            let query = req.uri().query().unwrap_or("");
            let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            let mut url_param = String::from("https://example.com");

            if let Some((_, value)) = params.iter().find(|(key, _)| key == "url") {
                let trimmed_val = value.trim();
                if value.starts_with("http") {
                    url_param = trimmed_val.to_string();
                } else {
                    url_param = String::from("https://") + trimmed_val
                }
            }

            // We can visit the upstream URL
            let parsed_url = url_param.parse::<hyper::Uri>();
            let preview_html = return_preview_html(parsed_url.unwrap().to_string(), client).await;
            build_response(StatusCode::OK, preview_html.unwrap())
        }
        _ => build_response(StatusCode::NOT_FOUND, String::from("Resource not found")),
    }
}

/// Follows redirects and returns the final resolved URL as a `String`.
///
/// For example, `https://youtu.be/...` will return `https://www.youtube.com/...`
///
/// # Errors
/// Returns an error if the request fails or the URL cannot be resolved.
async fn follow_endpoint(
    endpoint: String,
    client: Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let headers = build_headers(&endpoint);
    let resp = client
        .head(&endpoint)
        .headers(headers)
        .send()
        .await
        .map_err(|_er| format!("Failed to make Request to: {endpoint}"))?;

    Ok(resp.url().to_string())
}

async fn return_preview_html(
    endpoint: String,
    client: Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let headers = build_headers(&endpoint);
    let res = client.get(endpoint).headers(headers).send().await?;

    let html = res.text().await?;

    Ok(html)
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
