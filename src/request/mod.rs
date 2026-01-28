use std::{net::IpAddr, process, sync::Arc};

use log::error;
use reqwest::{Client, cookie::Jar};

/// Creates a configured `reqwest::Client` that binds to `0.0.0.0` and uses a shared cookie jar.
///
/// Returns the constructed `Client`. If the client builder fails, logs an error and terminates the process with exit code `1`.
///
/// # Examples
///
/// ```
/// let client = create_reqwest();
/// // use `client` to send requests, e.g. `client.get("https://example.com").send().await`
/// ```
pub fn create_reqwest() -> Client {
    let cookie_store = Arc::new(Jar::default());

    match Client::builder()
        .local_address(IpAddr::from([0, 0, 0, 0]))
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            error!("Failed to intitialize reqwest client.");
            process::exit(1)
        }
    }
}