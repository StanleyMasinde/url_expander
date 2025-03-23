pub mod expander;

use std::{env::args, net::SocketAddr};

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
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

    let listener = TcpListener::bind(addr).await?;

    println!("Server running on http://localhost:{}", addr.port());

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn::<_, _, _>(expander::handle_expansion))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
