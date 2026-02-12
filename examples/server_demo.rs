use httpx_transport::HttpxServer;
use httpx_core::ServerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // The DX Promise: 0-RTT, Intent-Aware Server in 10 lines.
    let mut config = ServerConfig::default();
    config.slab_capacity = 128;
    config.threads = 1;

    HttpxServer::listen("127.0.0.1:8080")
        .with_config(config)
        .with_intent_predicting()
        .start()
        .await?;

    Ok(())
}
