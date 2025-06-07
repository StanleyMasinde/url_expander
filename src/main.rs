mod expander;
use std::{
    env::args,
    net::{IpAddr, SocketAddr}, sync::Arc,
};

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use reqwest::{cookie::Jar, Client};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = args().collect();
    let binding = String::from("3000");
    let port = args.get(1).unwrap_or(&binding);

    let port_number = port
        .parse::<u16>()
        .map_err(|e| format!("The port has to be a number: {:?}", e))?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port_number));

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}", e))?;

    println!("Server running on http://localhost:{}", addr.port());

    // Create a cookie jar
    let cookie_store = Arc::new(Jar::default());
    let client_r = Client::builder()
        .local_address(IpAddr::from([0, 0, 0, 0]))
        .cookie_store(true)
        .cookie_provider(cookie_store.clone())
        .build()
        .unwrap();
    let http_builder = http1::Builder::new();

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let client = client_r.clone();
        let http_builder_ref = http_builder.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http_builder_ref
                .serve_connection(
                    io,
                    service_fn(move |svc| expander::handle_expansion(svc, client.clone())),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
