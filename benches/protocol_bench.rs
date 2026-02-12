use criterion::{black_box, criterion_group, criterion_main, Criterion};
use httpx_dsa::LinearIntentTrie;

fn trie_performance(c: &mut Criterion) {
    let mut trie = LinearIntentTrie::new(1024);
    trie.insert(b"intent_alpha");

    c.bench_function("linear_trie_lookup", |b| {
        b.iter(|| trie.exists(black_box(b"intent_alpha")))
    });
}

criterion_group!(benches, trie_performance);
criterion_main!(benches);
