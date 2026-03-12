use link_expander::server;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    server::run().await;
}
