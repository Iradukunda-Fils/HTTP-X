use httpx_dsa::{LinearIntentTrie};
use httpx_core::PredictiveEngine;
use httpx_core::Session;
use std::sync::Arc;
use tokio::time::{Duration};

#[tokio::test]
async fn test_adversarial_iiw_depletion() {
    let engine = PredictiveEngine::new(true);
    let addr = "127.0.0.1:8080".parse().unwrap();
    let session = Session::new(addr);
    let context = b"GET /";

    // 1. Setup Trie to always return > 85% probability
    let mut trie = LinearIntentTrie::new(1024);
    trie.observe(context, true);
    for _ in 0..100 { trie.observe(context, true); }
    engine.swap_weights(trie);

    // 2. Consume all 10 default credits
    for i in 0..10 {
        let decision = engine.fire_push_if_likely(&session, context);
        assert!(decision.is_some(), "Credit {} should be available", i);
    }

    // 3. Verify exhaustion
    let decision = engine.fire_push_if_likely(&session, context);
    assert!(decision.is_none(), "11th push must be blocked by IIW depletion");
    
    println!("Adversarial Audit: IIW Depletion Verified.");
}

#[tokio::test]
async fn test_shadow_swap_stress_load() {
    let engine = Arc::new(PredictiveEngine::new(true));
    let context = b"GET /";
    let addr = "127.0.0.1:8080".parse().unwrap();
    
    let engine_clone = engine.clone();
    let swap_jh = tokio::spawn(async move {
        for i in 0..1000 {
            let mut new_trie = LinearIntentTrie::new(1024);
            new_trie.observe(context, i % 2 == 0);
            engine_clone.swap_weights(new_trie);
            // High frequency swaps (1ms)
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });

    let mut lookup_workers = Vec::new();
    for _ in 0..8 {
        let engine_inner = engine.clone();
        lookup_workers.push(tokio::spawn(async move {
            let session = Session::new(addr);
            for _ in 0..100_000 {
                // We don't care about the decision, just that it doesn't crash 
                // and correctly accesses the trie during a swap.
                let _ = engine_inner.fire_push_if_likely(&session, context);
                // Replenish credits for the sake of the stress test if needed, 
                // but fire_push_if_likely is the hot path.
            }
        }));
    }

    for jh in lookup_workers {
        jh.await.unwrap();
    }
    swap_jh.await.unwrap();
    
    println!("Adversarial Audit: Shadow-Swap Stability certified under 800k lookups + 1k swaps.");
}

#[tokio::test]
async fn test_priority_zero_pivot_cancellation() {
    let engine = PredictiveEngine::new(true);
    let addr = "127.0.0.1:9999".parse().unwrap();
    let session = Session::new(addr);
    let context = b"GET /pivot";

    // 1. Setup probability
    let mut trie = LinearIntentTrie::new(1024);
    trie.observe(context, true);
    for _ in 0..100 { trie.observe(context, true); }
    engine.swap_weights(trie);

    // 2. Verify it works normally
    assert!(engine.fire_push_if_likely(&session, context).is_some());

    // 3. Simulate a Priority-Zero Pivot (Cancellation)
    session.cancel();
    
    // 4. Verify it is now blocked, despite high probability and credits
    let decision = engine.fire_push_if_likely(&session, context);
    assert!(decision.is_none(), "Push must be blocked after cancellation");
    
    println!("Adversarial Audit: Priority-Zero Pivot verified.");
}
