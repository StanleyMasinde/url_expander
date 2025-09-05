use reqwest::Client;

use crate::utils::build_headers::build_headers;

pub async fn expand_url(url: &str, client: Client) -> Result<String, reqwest::Error> {
    let final_url = follow_endpoint(url, client).await?;

    Ok(final_url)
}

/// Follows redirects and returns the final resolved URL as a `String`.
///
/// For example, `https://youtu.be/...` will return `https://www.youtube.com/...`
///
/// # Errors
/// Returns an error if the request fails or the URL cannot be resolved.
async fn follow_endpoint(endpoint: &str, client: Client) -> Result<String, reqwest::Error> {
    let headers = build_headers(endpoint);
    let resp = client.head(endpoint).headers(headers).send().await?;

    Ok(resp.url().to_string())
}
