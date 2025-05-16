pub mod response;

use http_body_util::combinators::BoxBody;
use hyper::{Error, Method, Request, Response, StatusCode, body::Bytes};
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

            let expanded_url = follow_endpoint(parsed_url.unwrap().to_string(), client).await;

            build_response(StatusCode::OK, expanded_url.unwrap())
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
    let resp = client
        .get(endpoint)
        .header(header::USER_AGENT, "curl/8.7.1")
        .send()
        .await?;

    Ok(resp.url().to_string())
}

async fn return_preview_html(
    endpoint: String,
    client: Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let user_agents = [
        "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/27.0 Chrome/125.0.0.0 Mobile Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/120.0.6099.119 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) EdgiOS/123.0.2420.56 Version/18.0 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 13; Redmi Note 12) AppleWebKit/537.36 (KHTML, like Gecko) Vivaldi/122.0 Mobile Safari/537.36",
        "Mozilla/5.0 (Linux; Android 10; HarmonyOS; BAH4-W09; HMSCore 6.15.0.302) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.93 HuaweiBrowser/11.1.2.332 Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/120.0.6099.119 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 9; Infinix X653C Build/PPR1.180610.011) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.6422.146 Mobile Safari/537.36",
        "Mozilla/5.0 (Linux; Android 12; V2234) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/87.0.4280.141 Mobile Safari/537.36 VivoBrowser/9.3.8.1",
    ];

    let user_agent = if endpoint.contains("facebook.com") || endpoint.contains("instagram.com") {
        "curl/8.7.1".to_string()
    } else {
        let mut rng = rng();
        let random_user_agent = user_agents.choose(&mut rng).unwrap();
        random_user_agent.to_string()
    };

    let res = client
        .get(endpoint)
        .header(header::USER_AGENT, user_agent)
        .header(
            header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        )
        .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .header(header::ACCEPT_ENCODING, "gzip, deflate, br")
        .send()
        .await?;

    let html = res.text().await?;

    Ok(html)
}
