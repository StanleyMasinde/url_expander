use rand::{rng, seq::IndexedRandom};

///
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
pub fn randomize_user_agent<'a>(endpoint: &str) -> &'a str {
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
        "curl/8.7.1"
    } else {
        let mut rng = rng();

        user_agents.choose(&mut rng).map_or("curl/8.7.1", |v| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_websites() {
        let user_agent = randomize_user_agent("https://stanleymasinde.com");
        assert!(user_agent.contains("Mozilla/5.0"));
    }

    #[test]
    fn meta_sites() {
        let user_agent = randomize_user_agent("https://instagram.com");
        assert!(user_agent.contains("curl/8"));
    }
}
