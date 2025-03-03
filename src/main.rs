pub mod expander;

use std::{env::args, net::SocketAddr};

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = args().collect();
    let binding = "3000".to_string();
    let port = args.get(1).unwrap_or(&binding);

    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse().unwrap()));

    let listener = TcpListener::bind(addr).await?;

    println!("Server running on http://localhost:{}", addr.port());

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            match http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn::<_, _, _>(expander::handle_expansion))
                .await
            {
                Err(err) => {
                    eprintln!("Error serving connection: {:?}", err);
                }
                _ => (),
            }
        });
    }
}
