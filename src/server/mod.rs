pub mod routes;
use axum::Router;
use reqwest::Client;
use std::{env::args, io::ErrorKind, process::exit};

#[derive(Clone)]
struct AppState {
    client: Client,
}

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
                eprintln!("You don't have persmission to port {port}.")
            } else if error.kind() == ErrorKind::AddrInUse {
                eprintln!("Port {port} is already in use.")
            } else {
                eprintln!("Could not start the server {error}")
            }

            exit(1)
        }
    };
    axum::serve(listener, app)
        .await
        .expect("Failed to start Axum.");
}
