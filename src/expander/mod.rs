pub mod response;

use http_body_util::combinators::BoxBody;
use hyper::{Error, Method, Request, Response, StatusCode, body::Bytes};
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

            let expanded_url = follow_endpoint(parsed_url.unwrap().to_string(), client).await;

            build_response(StatusCode::OK, expanded_url.unwrap())
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
    let resp = client
        .get(endpoint)
        .header(header::USER_AGENT, "curl/8.7.1")
        .send()
        .await?;

    Ok(resp.url().to_string())
}
