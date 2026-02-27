pub mod auth;
pub mod middleware;
pub mod routes;

use axum::Router;
use log::{error, warn};
use reqwest::Client;
use std::io::ErrorKind;

use crate::{auth::build_auth_service, types::Cache};

#[derive(Clone)]
struct AppState {
    client: Client,
    memory_cache: Cache,
}

pub async fn run() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let address = format!("127.0.0.1:{port}");

    let auth_service = match build_auth_service().await {
        Ok(service) => Some(service),
        Err(message) => {
            warn!("failed to initialize auth service: {}", message);
            warn!("auth routes are running in service-unavailable mode");
            None
        }
    };

    let app: Router = routes::routes_with_auth(auth_service);

    let listener = match tokio::net::TcpListener::bind(address)
        .await
        .inspect(|t| println!("Server started on http://{}", t.local_addr().unwrap()))
    {
        Ok(l) => l,
        Err(error) => {
            if error.kind() == ErrorKind::PermissionDenied {
                error!("You don't have permission to port {port}.")
            } else if error.kind() == ErrorKind::AddrInUse {
                error!("Port {port} is already in use.")
            } else {
                error!("Could not start the server {error}")
            }

            std::process::exit(1)
        }
    };

    axum::serve(listener, app)
        .await
        .expect("Failed to start Axum.");
}
