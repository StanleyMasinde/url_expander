pub mod middleware;
pub mod routes;
use axum::Router;
use log::error;
use reqwest::Client;
use std::{env::args, io::ErrorKind, process::exit};

use crate::utils::{cache::Cache, job_runner::job_runner};

#[derive(Clone)]
struct AppState {
    client: Client,
    memory_cache: Cache,
}

/// Start the HTTP server bound to 127.0.0.1 on the port specified by the first command-line
/// argument or `3000` when no argument is provided.
///
/// The function spawns the background `job_runner`, constructs the application router from
/// `routes::routes()`, and attempts to bind a TCP listener to the chosen address. On successful
/// bind it prints the server address to stdout. If binding fails due to permission or address-in-use
/// errors it logs a descriptive message and exits the process with code 1.
///
/// # Examples
///
/// ```
/// // Start the server using the default port (3000) when running the binary.
/// // Note: in real usage this runs indefinitely serving requests.
/// #[tokio::main]
/// async fn main() {
///     my_crate::run().await;
/// }
/// ```
pub async fn run() {
    let default_port = String::from("3000");
    let args: Vec<String> = args().collect();
    let port = args.get(1).unwrap_or(&default_port);
    let address = format!("127.0.0.1:{port}");

    let app = Router::new().merge(routes::routes());

    tokio::spawn(job_runner());

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