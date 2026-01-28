use std::{net::IpAddr, process, sync::Arc};

use log::error;
use reqwest::{Client, cookie::Jar};

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
