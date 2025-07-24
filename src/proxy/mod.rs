use reqwest::Client;

use crate::utils::build_headers;

///
/// Returns a full html page. This is ideal for trying to
/// Render SEO previews on clients from Browsers.
/// CORS is not so friendly
///
pub async fn return_preview_html(
    endpoint: String,
    client: Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let headers = build_headers(&endpoint);
    let res = client.get(endpoint).headers(headers).send().await?;

    let html = res.text().await?;

    Ok(html)
}
