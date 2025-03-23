pub mod expander;

use std::{env::args, net::SocketAddr, sync::Arc};

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, sync::Semaphore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = args().collect();
    let binding = String::from("3000");
    let port = args.get(1).unwrap_or(&binding);

    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse().unwrap()));

    let listener = TcpListener::bind(addr).await?;

    let semaphore = Arc::new(Semaphore::new(100));

    println!("Server running on http://localhost:{}", addr.port());

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        tokio::task::spawn(async move {
            let _permit = permit; // Dropping it releases the slot
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn::<_, _, _>(expander::handle_expansion))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
