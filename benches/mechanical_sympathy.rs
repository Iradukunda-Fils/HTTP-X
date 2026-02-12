use criterion::{black_box, criterion_group, criterion_main, Criterion};
use httpx_dsa::LinearIntentTrie;

fn bench_mechanical_perf(c: &mut Criterion) {
    let trie = LinearIntentTrie::new(1024);
    let key = b"predictive_context";

    let mut group = c.benchmark_group("mechanical_sympathy");
    
    // CPP (Cycles Per Packet) Audit
    group.bench_function("trie_traversal_cpp", |b| {
        b.iter(|| {
            // Hallucination Check: L2 Cache Latency
            // Target: < 1000 cycles.
            black_box(trie.get_probability(key, true));
        })
    });

    group.finish();
}

criterion_group!(benches, bench_mechanical_perf);
criterion_main!(benches);
