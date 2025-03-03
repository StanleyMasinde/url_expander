use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Bytes, Error, Method, Request, Response, StatusCode};
use reqwest::{header, Client};

pub async fn handle_expansion(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, Error>>, Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let query = req.uri().query().unwrap_or("");

            let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            let mut url_param = "".to_string();

            if let Some((_, value)) = params.iter().find(|(key, _)| key == "url") {
                url_param = value.to_string()
            }

            let mut expanded_url = "".to_string();

            // We can visit the upstream URL
            if !url_param.is_empty() {
                let parsed_url = url_param.parse::<hyper::Uri>().unwrap();
                expanded_url = match follow_endpoint(parsed_url.to_string()).await {
                    Ok(ok_res) => ok_res,
                    Err(_err) => "err".to_string(),
                };
            }

            if expanded_url.is_empty() {
                let mut bad_request = Response::new(full("URL param missing!"));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;

                return Ok(bad_request);
            }

            if expanded_url == "err" {
                let mut unproceacable = Response::new(full("Invalid URL"));
                *unproceacable.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;

                return Ok(unproceacable);
            }

            Ok(Response::new(full(expanded_url.to_string())))
        }
        _ => {
            let mut not_found = Response::new(empty(req.uri().path()));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn follow_endpoint(endpoint: String) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let resp = client.get(endpoint)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        .header(header::REFERER, "https://www.google.com/")
        .send().await?;

    Ok(resp.url().clone().to_string())
}

fn empty(path: &str) -> BoxBody<Bytes, Error> {
    Full::new(Bytes::from(format!(
        "Resource {} not found on this server. That's all we know.",
        path
    )))
    .map_err(|never| match never {})
    .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
