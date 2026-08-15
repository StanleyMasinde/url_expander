use reqwest::Client;

use crate::{types::OEmbedResponse, utils::build_headers::build_headers};

/// Fetches and returns the HTML content of a given endpoint.
///
/// Returns a full html page. This is ideal for trying to
/// Render SEO previews on clients from Browsers.
/// CORS is not so friendly
pub async fn return_preview_html(endpoint: &str, client: Client) -> Result<String, reqwest::Error> {
    let headers = build_headers(endpoint);
    let res = client.get(endpoint).headers(headers).send().await?;

    let html = res.text().await?;

    Ok(html)
}

pub async fn return_youtube_preview(
    video_url: &str,
    client: Client,
) -> Result<OEmbedResponse, reqwest::Error> {
    let endpoint = "https://www.youtube.com/oembed";
    let params = [("url", video_url), ("format", "json")];

    let json = client
        .get(endpoint)
        .query(&params)
        .send()
        .await?
        .json::<OEmbedResponse>()
        .await?;

    Ok(json)
}

#[derive(Debug)]
pub enum RedditPreviewError {
    Transport(reqwest::Error),
    Upstream(String),
}

impl From<reqwest::Error> for RedditPreviewError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport(error)
    }
}

fn is_reddit_share_url(url: &str) -> bool {
    url.contains("reddit.com") && url.contains("/s/")
}

fn strip_query(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(mut parsed) => {
            parsed.set_query(None);
            parsed.set_fragment(None);
            parsed.to_string()
        }
        Err(_) => url.to_string(),
    }
}

/// Share links (/r/sub/s/xxx) 301 to /comments/. Resolve those first.
async fn resolve_reddit_post_url(
    post_url: &str,
    client: &Client,
) -> Result<String, reqwest::Error> {
    if !is_reddit_share_url(post_url) && !post_url.contains("://redd.it/") {
        return Ok(strip_query(post_url));
    }

    let headers = build_headers(post_url);
    let resp = client.get(post_url).headers(headers).send().await?;
    Ok(strip_query(resp.url().as_str()))
}

/// Reddit's HTML pages are a WAF wall. Fetch oEmbed with the post URL instead.
pub async fn return_reddit_preview(
    post_url: &str,
    client: Client,
) -> Result<OEmbedResponse, RedditPreviewError> {
    let resolved = resolve_reddit_post_url(post_url, &client).await?;
    let endpoint = "https://www.reddit.com/oembed";
    let params = [("url", resolved.as_str())];
    let headers = build_headers(endpoint);

    let response = client
        .get(endpoint)
        .headers(headers)
        .query(&params)
        .send()
        .await?;

    let body = response.text().await?;
    serde_json::from_str::<OEmbedResponse>(&body).map_err(|_| {
        RedditPreviewError::Upstream(format!(
            "Reddit oEmbed did not return JSON for {resolved}: {}",
            body.chars().take(120).collect::<String>()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_youtube_embed() {
        let video_url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let client = reqwest::Client::new();

        let res = return_youtube_preview(video_url, client).await.unwrap();

        assert!(!res.title.is_empty(), "The title should not be empty");
        assert!(
            !res.author_name.is_empty(),
            "The author name should not be empty"
        );

        assert_eq!(res.author_name, "Rick Astley", "Channel name mismatch");
        assert!(
            res.title.contains("Never Gonna Give You Up"),
            "Title did not contain expected string. Got: {}",
            res.title
        );

        assert!(
            res.thumbnail_url.starts_with("https://i.ytimg.com/"),
            "Thumbnail domain is unexpected. Got: {}",
            res.thumbnail_url
        );
        assert!(
            res.html.contains("iframe"),
            "The embed HTML snippet should contain an iframe wrapper"
        );

        println!(
            "Verified metadata for video: \"{}\" by {}",
            res.title, res.author_name
        );
    }

    #[tokio::test]
    async fn test_youtube_embed_short_url() {
        let video_url = "https://youtu.be/dQw4w9WgXcQ?si=ZR4vyAMFMlq1QSiI";
        let client = reqwest::Client::new();

        let res = return_youtube_preview(video_url, client).await.unwrap();

        assert!(!res.title.is_empty(), "The title should not be empty");
        assert!(
            !res.author_name.is_empty(),
            "The author name should not be empty"
        );

        assert_eq!(res.author_name, "Rick Astley", "Channel name mismatch");
        assert!(
            res.title.contains("Never Gonna Give You Up"),
            "Title did not contain expected string. Got: {}",
            res.title
        );

        assert!(
            res.thumbnail_url.starts_with("https://i.ytimg.com/"),
            "Thumbnail domain is unexpected. Got: {}",
            res.thumbnail_url
        );
        assert!(
            res.html.contains("iframe"),
            "The embed HTML snippet should contain an iframe wrapper"
        );

        println!(
            "Verified metadata for video: \"{}\" by {}",
            res.title, res.author_name
        );
    }

    #[tokio::test]
    async fn test_reddit_embed() {
        let post_url = "https://www.reddit.com/r/pics/comments/92dd8/test_post_please_ignore/";
        let client = reqwest::Client::new();

        let res = return_reddit_preview(post_url, client).await.unwrap();

        assert_eq!(res.title, "test post please ignore");
        assert_eq!(res.author_name, "qgyh2");
        assert!(
            res.html.contains("reddit-embed-bq") || res.html.contains("reddit"),
            "Embed HTML should mention Reddit. Got: {}",
            res.html
        );
    }

    #[test]
    fn test_reddit_share_url_detection() {
        assert!(is_reddit_share_url(
            "https://www.reddit.com/r/node/s/cv5XKIpUIr"
        ));
        assert!(!is_reddit_share_url(
            "https://www.reddit.com/r/node/comments/1vp379b/why_is_javascript_criticized_so_much_for_backend/"
        ));
    }

    #[test]
    fn test_strip_query_keeps_comments_path() {
        let dirty = "https://www.reddit.com/r/node/comments/1vp379b/title/?share_id=abc&utm_source=share";
        assert_eq!(
            strip_query(dirty),
            "https://www.reddit.com/r/node/comments/1vp379b/title/"
        );
    }
}
