use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Bytes, header::HeaderValue, Error, Method, Request, Response, StatusCode};
use reqwest::{header, Client};

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
                let mut bad_request = Response::new(full("Failed to parse URL. Invalid URL"));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                hyper::HeaderMap::insert(
                    bad_request.headers_mut(),
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );

                return Ok(bad_request);
            }

            let expanded_url = follow_endpoint(parsed_url.unwrap().to_string(), client).await;

            if expanded_url.is_err() {
                let mut bad_request = Response::new(full("URL param missing!"));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                hyper::HeaderMap::insert(
                    bad_request.headers_mut(),
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );

                return Ok(bad_request);
            }

            let mut ok_response = Response::new(full(expanded_url.unwrap()));
            hyper::HeaderMap::insert(
                ok_response.headers_mut(),
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_static("*"),
            );

            Ok(ok_response)
        }
        _ => {
            let mut not_found = Response::new(empty(req.uri().path()));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
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

    Ok(resp.url().clone().to_string())
}

/// Return 404 not found response
fn empty(path: &str) -> BoxBody<Bytes, Error> {
    Full::new(Bytes::from(format!(
        "Resource {} not found on this server. That's all we know.",
        path
    )))
    .map_err(|never| match never {})
    .boxed()
}

/// Return full response a 200
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
