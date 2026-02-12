pub mod config;
pub mod error;
pub mod registry;
pub mod bridge;
pub mod engine;
pub mod session;

pub use config::ServerConfig;
pub use engine::PredictiveEngine;
pub use session::{Session, SessionMode};
pub use error::HttpXError;
pub use registry::ResourceRegistry;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ControlSignal {
    Pivot(SocketAddr),
    KillAll,
    SwapTrie(Arc<httpx_dsa::LinearIntentTrie>),
}

/// A unified builder for Sovereign HTTP-X servers.
/// 
/// ## Mechanical Sympathy: Static Registration
/// Routes are "burned" into the trie during the build phase, 
/// ensuring O(1) matching in the data plane.
pub struct ServerBuilder {
    pub registry: ResourceRegistry,
    pub config: ServerConfig,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            registry: ResourceRegistry::new(),
            config: ServerConfig::default(),
        }
    }

    /// Registers a route with a pre-allocated payload handle.
    pub fn route(mut self, path: &str, handle: u32, version: u32) -> Self {
        self.registry.route(path, handle, version);
        self
    }

    /// Overrides the default server configuration.
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_production_mode(mut self, enabled: bool) -> Self {
        self.config.production_mode = enabled;
        self
    }
}
