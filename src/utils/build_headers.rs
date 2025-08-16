use axum::http::{HeaderMap, HeaderValue};
use reqwest::header;

use crate::utils::rand_ua::randomize_user_agent;

/// Panics if:
/// - The `randomize_user_agent` function panics due to an empty list of User-Agent strings or a failure in random selection.
/// - The `HeaderValue::from_str` method panics if the provided User-Agent string is invalid.
/// - The `HeaderValue::from_static` method panics if the provided static string is invalid (unlikely with hardcoded values).
pub fn build_headers(endpoint: &str) -> HeaderMap {
    let user_agent = randomize_user_agent(endpoint);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static(user_agent),
    );
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        ),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.5"),
    );
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));

    headers.insert(
        header::REFERER,
        HeaderValue::from_static("https://www.google.com/"),
    );

    headers
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_build_headers() {
        let headers = build_headers("https://stanleymasinde.com");
        let cache_control = headers.get(header::CACHE_CONTROL).unwrap();


        assert_eq!(cache_control, HeaderValue::from_static("no-cache"));
    }
}
