mod expander;
mod proxy;
mod request;
mod server;
mod utils;

#[tokio::main]
async fn main() {
    server::run().await;
}
