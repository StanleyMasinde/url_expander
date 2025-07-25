use axum::http::{HeaderMap, HeaderValue};
use rand::{rng, seq::IndexedRandom};
use reqwest::{StatusCode, header};

/// Generates a randomized User-Agent string based on the provided endpoint.
///
/// This function selects a User-Agent string from a predefined list of common User-Agent headers.
/// If the endpoint contains "facebook.com" or "instagram.com", a specific User-Agent string
/// ("curl/8.7.1") is returned instead.
///
/// # Parameters
/// - `endpoint`: A string slice representing the target endpoint URL.
///
/// # Returns
/// A `String` containing the selected User-Agent header.
///
/// # Panics
/// Panics if the predefined list of User-Agent strings is empty or if the random selection fails.
pub fn randomize_user_agent(endpoint: &str) -> String {
    let user_agents = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edg/113.0.1774.50 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_4) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/113.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 12_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/113.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_2_3) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.82 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Brave/113.0.5672.126 Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Vivaldi/5.6.2867.58 Chrome/113.0.5672.126 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.3 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:93.0) Gecko/20100101 Firefox/93.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/94.0.4606.61 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Opera/80.0.4170.63 Safari/537.36",
    ];
    if endpoint.contains("facebook.com") || endpoint.contains("instagram.com") {
        "curl/8.7.1".to_string()
    } else {
        let mut rng = rng();
        let random_user_agent = user_agents.choose(&mut rng).unwrap();
        random_user_agent.to_string()
    }
}

/// Panics if:
/// - The `randomize_user_agent` function panics due to an empty list of User-Agent strings or a failure in random selection.
/// - The `HeaderValue::from_str` method panics if the provided User-Agent string is invalid.
/// - The `HeaderValue::from_static` method panics if the provided static string is invalid (unlikely with hardcoded values).
pub fn build_headers(endpoint: &str) -> HeaderMap {
    let user_agent = randomize_user_agent(endpoint);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_str(&user_agent).expect("Invalid user agent"),
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
        HeaderValue::from_str("https://www.google.com/").unwrap(),
    );

    headers
}

/// Handles all `reqwest` errors by categorizing them based on their kind.
///
/// This function provides exhaustive error handling and replaces the use of `Box<dyn Error>`.
/// It determines the exact error type and returns an appropriate HTTP status code along with
/// a descriptive error message.
///
/// # Parameters
/// - `error`: A `reqwest::Error` instance representing the error to handle.
///
/// # Returns
/// A tuple containing:
/// - `reqwest::StatusCode`: The HTTP status code corresponding to the error type.
/// - `String`: A descriptive error message.
///
pub fn handle_reqwest_error(error: reqwest::Error) -> (reqwest::StatusCode, std::string::String) {
    if error.is_builder() {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "The provided URL is not valid. Please check it and try again.".to_string(),
        )
    } else if error.is_request() {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "Request failed! {} does not seem to resolve to a valid domain.",
                error
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown URL".to_string())
            ),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "An error occurred on our side. Please try again later.".to_string(),
        )
    }
}
