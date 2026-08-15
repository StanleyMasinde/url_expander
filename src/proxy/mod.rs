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

/// Reddit's HTML pages are a WAF wall. Fetch oEmbed with the post URL instead.
pub async fn return_reddit_preview(
    post_url: &str,
    client: Client,
) -> Result<OEmbedResponse, reqwest::Error> {
    let endpoint = "https://www.reddit.com/oembed";
    let params = [("url", post_url)];
    let headers = build_headers(endpoint);

    let json = client
        .get(endpoint)
        .headers(headers)
        .query(&params)
        .send()
        .await?
        .json::<OEmbedResponse>()
        .await?;

    Ok(json)
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
}
