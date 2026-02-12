use criterion::{criterion_group, criterion_main, Criterion};
use httpx_dsa::{LinearIntentTrie, SecureSlab};
use httpx_transport::dispatcher::CoreDispatcher;
use httpx_core::ServerConfig;
use tokio::net::UdpSocket;
use std::time::Instant;

fn bench_payload_propagation(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut trie = LinearIntentTrie::new(1024);
    let context = b"GET /index.html";
    trie.observe(context, true);
    trie.associate_payload(context, 42, 0);

    let slab = SecureSlab::new(64);
    let socket = rt.block_on(UdpSocket::bind("127.0.0.1:0")).unwrap();
    let addr = socket.local_addr().unwrap();
    let (_tx, rx) = tokio::sync::mpsc::channel(10);
    
    let mut dispatcher = rt.block_on(CoreDispatcher::new_with_socket(0, socket, rx, ServerConfig::default(), trie.clone())).unwrap();

    c.benchmark_group("Payload Fast-Path")
        .bench_function("Trie-to-Wire Latency", |b| {
            b.iter(|| {
                let start = Instant::now();
                
                // 1. Simulate Trie Lookup Hit
                let node = trie.get_node(1).unwrap();
                let handle = node.payload_handle;
                let version = node.version_id;
                
                // 2. Atomic Submission
                rt.block_on(dispatcher.submit_linked_burst(addr, handle, 0, version, &slab)).unwrap();
                
                let duration = start.elapsed();
                // # Mechanical Sympathy Target: < 8µs
                if duration.as_micros() > 8 {
                    // In a real bench, we'd record this as a violation
                }
            })
        });
}

criterion_group!(benches, bench_payload_propagation);
criterion_main!(benches);
