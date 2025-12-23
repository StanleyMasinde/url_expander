use std::net::SocketAddr;

use axum::extract::Request;
use sha2::{Digest, Sha256};

pub fn generate_fingerprint<B>(request: &Request<B>) -> String {
    let ip = request
        .extensions()
        .get::<SocketAddr>()
        .map(|a| a.ip().to_string())
        .unwrap_or_default();

    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut hasher = Sha256::new();
    hasher.update(ip);
    hasher.update(user_agent);

    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod test {
    use axum::extract::Request;
    use pretty_assertions::assert_eq;

    use crate::utils::fingerprint::generate_fingerprint;

    #[test]
    fn test_generate_fingerprint() {
        let request = Request::new("Hello, world");
        let fingerprint = generate_fingerprint(&request);

        assert_eq!(fingerprint.is_empty(), false);
    }
}
