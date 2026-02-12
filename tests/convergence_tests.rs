use httpx_core::{PredictiveEngine, Session};
use std::net::SocketAddr;

#[test]
fn test_engine_initialization() {
    let engine = PredictiveEngine::new(true);
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let session = Session::new(addr);
    
    // Basic verification of the predictive path
    let context = [0u8; 4];
    let _ = engine.fire_push_if_likely(&session, &context);
}
