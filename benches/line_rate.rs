use criterion::{black_box, criterion_group, criterion_main, Criterion};
use httpx_dsa::LinearIntentTrie;
use httpx_transport::{engine::PredictiveEngine, session::Session};
use std::net::SocketAddr;
use std::sync::Arc;

fn bench_contended_lookup(c: &mut Criterion) {
    let engine = Arc::new(PredictiveEngine::new(true));
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let session = Session::new(addr);
    let key = b"predictive_context";

    // Background thread performing rapid weight swaps
    let e2 = engine.clone();
    std::thread::spawn(move || {
        loop {
            let new_trie = LinearIntentTrie::new(1024);
            e2.swap_weights(new_trie);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    c.bench_function("contended_shadow_swap_lookup", |b| {
        b.iter(|| {
            // # Verification: Prove < 3ns lookup during swap.
            // Acquire-load overhead should be minimal (< 1ns).
            black_box(engine.fire_push_if_likely(&session, key));
        })
    });
}

criterion_group!(benches, bench_contended_lookup);
criterion_main!(benches);
