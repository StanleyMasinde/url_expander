use axum::http::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, header};

use crate::utils::rand_ua::randomize_user_agent;
pub mod rand_ua;

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
