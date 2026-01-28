use std::{net::IpAddr, process, sync::Arc};

use log::error;
use reqwest::{Client, cookie::Jar};

/// Creates and returns a preconfigured `reqwest::Client`.
///
/// The client is configured with a cookie jar (shared `Arc<Jar>`), `cookie_store` enabled,
/// and `local_address` bound to `0.0.0.0`. If the client cannot be constructed the process
/// will terminate with exit code 1.
///
/// # Returns
///
/// A configured `reqwest::Client`.
///
/// # Examples
///
/// ```
/// let client = create_reqwest();
/// // use `client` to send requests...
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
            error!("Failed to initialise reqwest client.");
            process::exit(1)
        }
    }
}