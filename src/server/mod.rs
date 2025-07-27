pub mod routes;
use axum::Router;
use reqwest::Client;
use std::env::args;

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

    println!("Server running on http://{}", &address);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
