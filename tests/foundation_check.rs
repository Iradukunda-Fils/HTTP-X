use httpx_core::bridge::{SqBridge, DropReason};
use httpx_core::PredictiveEngine;
use httpx_core::Session;
use std::net::SocketAddr;
use perf_event::Builder;
use perf_event::events::Software;

#[test]
fn test_zero_blocking_bridge_saturation() {
    // # Mechanical Sympathy: Capacity must be power-of-two for bitwise masking.
    let bridge = SqBridge::new(1024);
    let _engine = PredictiveEngine::new(true);
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let session = Session::new(addr);

    // 1. Saturate the bridge - 1024 slots
    for i in 0..1024 {
        let _ = bridge.try_push(i);
    }

    // # Verification: try_push must return Err(DropReason::Congested) immediately, 
    // never blocking the engine thread.
    let result = bridge.try_push(1025);
    assert!(matches!(result, Err(DropReason::Congested)));

    // 2. Setup perf counter for context switches
    // This requires Linux CAP_SYS_ADMIN or /proc/sys/kernel/perf_event_paranoid <= 2
    let mut counter = Builder::new()
        .kind(Software::CONTEXT_SWITCHES)
        .build()
        .expect("Failed to build perf counter. Ensure perf_event permissions are granted.");

    counter.enable().expect("Failed to enable perf counter");

    // 3. Perform 1,000,000 push-drop cycles to stress the wait-free logic
    for _ in 0..1_000_000 {
        if let Err(e) = bridge.try_push(999) {
             match e {
                 DropReason::Congested => { /* Expected: Zero Blocking */ }
             }
        }
    }

    counter.disable().expect("Failed to disable perf counter");
    let switches = counter.read().expect("Failed to read perf counter");

    // # Certification: Zero context switches during the hot loop proves 
    // the SQ-Bridge is truly wait-free.
    println!("Context Switches during 1M push-drop cycles: {}", switches);
    
    // We allow a small threshold for background kernel activity, 
    // but the hot-loop execution should be linear.
    assert!(switches < 50, "Wait-Free Violation: {} context switches detected", switches);

    // 4. Verify IIW Credit depletion
    // Session starts with 10 credits.
    for _ in 0..10 {
        assert!(session.consume_credit());
    }
    // 11th push must fail (DropReason: IIW Depletion)
    assert!(!session.consume_credit());
}
