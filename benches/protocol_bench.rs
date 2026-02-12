use criterion::{black_box, criterion_group, criterion_main, Criterion};
use httpx_dsa::LinearIntentTrie;

fn trie_performance(c: &mut Criterion) {
    let mut trie = LinearIntentTrie::new(1024);
    trie.observe(b"intent_alpha", true);

    c.bench_function("linear_trie_lookup", |b| {
        b.iter(|| trie.get_node_at_path(black_box(b"intent_alpha")).is_some())
    });
}

criterion_group!(benches, trie_performance);
criterion_main!(benches);
