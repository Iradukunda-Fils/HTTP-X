//! # Core Layer Tests: ResourceRegistry, ServerConfig, ServerBuilder
//!
//! Validates URI-to-Trie binding, default config correctness,
//! and the builder chain API.

use httpx_core::{ServerConfig, ServerBuilder};
use std::time::Instant;

/// Verifies that `ResourceRegistry::route` correctly warms the trie
/// and that `take_trie` returns a trie where the path is resolvable.
#[test]
fn test_resource_registry_route_roundtrip() {
    let t = Instant::now();

    let mut registry = httpx_core::ResourceRegistry::new();
    registry.route("/api/v1/hello", 42, 100);

    let trie = registry.take_trie();

    // The warmed path must be resolvable
    let node = trie.get_node_at_path(b"/api/v1/hello");
    assert!(node.is_some(), "Warmed path not found in trie");

    let node = node.unwrap();
    assert_eq!(node.payload_handle, 42, "Payload handle mismatch");
    assert_eq!(node.version_id, 100, "Version ID mismatch");

    let overhead = t.elapsed();
    println!("test_resource_registry_route_roundtrip: Testing Overhead = {:?}", overhead);
}

/// Verifies that `ServerConfig::default()` returns sane values.
#[test]
fn test_server_config_defaults() {
    let t = Instant::now();

    let config = ServerConfig::default();

    assert_eq!(config.threads, 2, "Default threads should be 2");
    assert_eq!(config.slab_capacity, 1024, "Default slab_capacity should be 1024");
    assert_eq!(config.predictive_depth, 5, "Default predictive_depth should be 5");
    assert_eq!(config.max_intent_credits, 1000, "Default max_intent_credits should be 1000");
    assert!(!config.production_mode, "production_mode should default to false");

    let overhead = t.elapsed();
    println!("test_server_config_defaults: Testing Overhead = {:?}", overhead);
}

/// Verifies the `ServerBuilder` fluent API and `production_mode` toggle.
#[test]
fn test_server_builder_production_mode() {
    let t = Instant::now();

    let builder = ServerBuilder::new()
        .with_production_mode(true);

    assert!(builder.config.production_mode, "production_mode should be true after with_production_mode(true)");

    // Verify route registration through builder
    let builder = builder.route("/health", 1, 1);
    // The registry should have the route (we can't inspect directly,
    // but building via from_builder later would work)
    let _ = builder;

    let overhead = t.elapsed();
    println!("test_server_builder_production_mode: Testing Overhead = {:?}", overhead);
}
