use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub threads: usize,
    pub max_intent_credits: u32,
    pub predictive_depth: usize,
    pub slab_capacity: usize,
    pub production_mode: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            threads: 2,
            max_intent_credits: 1000,
            predictive_depth: 5,
            slab_capacity: 1024,
            production_mode: false,
        }
    }
}
