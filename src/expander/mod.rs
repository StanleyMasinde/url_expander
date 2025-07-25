use std::error::Error;

use axum::http::Uri;
use reqwest::Client;

use crate::utils::build_headers;

pub async fn expand_url(url: String, client: Client) -> Result<String, Box<dyn Error>> {
    let parsed_url = url.parse::<Uri>().unwrap();

    let final_url = follow_endpoint(parsed_url.to_string(), client).await?;

    Ok(final_url)
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
