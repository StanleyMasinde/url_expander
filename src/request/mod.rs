use std::{net::IpAddr, sync::Arc};

use reqwest::{Client, cookie::Jar};

pub fn create_reqwest() -> Client {
    let cookie_store = Arc::new(Jar::default());

    Client::builder()
        .local_address(IpAddr::from([0, 0, 0, 0]))
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
        .unwrap()
}
