use std::{convert::Infallible, net::SocketAddr};

use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, world"))))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    println!("Server running on http://localhost:{}", addr.port());

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            match http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn::<_, _, _>(hello))
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
