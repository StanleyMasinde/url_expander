pub mod middleware;
pub mod routes;
use axum::Router;
use log::error;
use reqwest::Client;
use std::{env::args, io::ErrorKind, process::exit};

use crate::types::Cache;

#[derive(Clone)]
struct AppState {
    client: Client,
    memory_cache: Cache,
}

/// Starts the Axum HTTP server using the application's routes and a port taken from the first command-line argument (defaults to 3000).
///
/// The function binds a TCP listener on 127.0.0.1:<port>, prints the bound address on success, logs and exits the process with code 1 on bind failures, and runs the Axum router returned by `routes::routes()`.
///
/// # Examples
///
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     run().await;
/// }
/// ```
pub async fn run() {
    let default_port = String::from("3000");
    let args: Vec<String> = args().collect();
    let port = args.get(1).unwrap_or(&default_port);
    let address = format!("127.0.0.1:{port}");

    let app = Router::new().merge(routes::routes());

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

            exit(1)
        }
    };
    axum::serve(listener, app)
        .await
        .expect("Failed to start Axum.");
}