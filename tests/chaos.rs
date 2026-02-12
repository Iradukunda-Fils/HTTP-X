use httpx_core::{ControlSignal, PredictiveEngine};
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_pivot_stressor_latency() {
    let (tx, mut rx) = mpsc::channel(100);
    let engine = Arc::new(PredictiveEngine::new(true));
    let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();

    // Spawn a simulated Priority-Zero interceptor
    tokio::spawn(async move {
        while let Some(signal) = rx.recv().await {
            match signal {
                ControlSignal::Pivot(a) => {
                    let start = std::time::Instant::now();
                    engine.cancel_for(&a);
                    let elapsed = start.elapsed();
                    
                    // Hardware Requirement: < 100μs
                    assert!(elapsed.as_micros() < 100, "Pivot cancellation too slow: {}μs", elapsed.as_micros());
                }
                _ => {}
            }
        }
    });

    // Stress test: 1000 rapid pivots
    for _ in 0..1000 {
        tx.send(ControlSignal::Pivot(addr)).await.unwrap();
    }
}
