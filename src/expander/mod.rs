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
fn is_reddit_url(endpoint: &str) -> bool {
    endpoint.contains("reddit.com") || endpoint.contains("redd.it")
}

async fn follow_endpoint(endpoint: &str, client: Client) -> Result<String, reqwest::Error> {
    let headers = build_headers(endpoint);
    // Reddit share links (/r/sub/s/...) often omit Location on HEAD.
    let resp = if is_reddit_url(endpoint) {
        client.get(endpoint).headers(headers).send().await?
    } else {
        client.head(endpoint).headers(headers).send().await?
    };

    Ok(resp.url().to_string())
}
