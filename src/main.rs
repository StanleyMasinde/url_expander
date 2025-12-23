mod expander;
mod proxy;
mod request;
mod server;
mod types;
mod utils;

#[tokio::main]
async fn main() {
    env_logger::init();
    server::run().await;
}
