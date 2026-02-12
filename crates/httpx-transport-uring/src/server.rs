use httpx_core::config::ServerConfig;
use crate::engine::PredictiveEngine;
use std::net::SocketAddr;

pub struct UringServer {
    addr: SocketAddr,
    config: ServerConfig,
}

impl UringServer {
    pub fn bind(addr: &str) -> UringServerBuilder {
        UringServerBuilder {
            addr: addr.parse().expect("Invalid address"),
            config: ServerConfig::default(),
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let _engine = PredictiveEngine::new();
        // io_uring loop logic would live here
        Ok(())
    }
}

pub struct UringServerBuilder {
    addr: SocketAddr,
    config: ServerConfig,
}

impl UringServerBuilder {
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_intent_predict(self) -> Self {
        // Toggle engine behavioral analysis
        self
    }

    pub fn build(self) -> Result<UringServer, Box<dyn std::error::Error>> {
        Ok(UringServer {
            addr: self.addr,
            config: self.config,
        })
    }
}
