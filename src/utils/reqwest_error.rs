use reqwest::StatusCode;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reqwest_error_handling() {
        let error: reqwest::Error = {};
        let response = handle_reqwest_error(error);
    }
}
