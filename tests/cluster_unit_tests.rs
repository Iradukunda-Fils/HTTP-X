//! # Cluster Layer Tests: ReconciliationBuffer
//!
//! Validates the offline learning buffer's record, merge, and clear lifecycle.

use httpx_cluster::ReconciliationBuffer;
use httpx_dsa::LinearIntentTrie;
use std::time::Instant;

/// Verifies the record → merge → clear lifecycle.
#[test]
fn test_reconciliation_buffer_record_and_clear() {
    let t = Instant::now();

    let mut buffer = ReconciliationBuffer::new();

    // Record some learning events
    buffer.record(0xDEADBEEF, true);
    buffer.record(0xDEADBEEF, true);
    buffer.record(0xDEADBEEF, false);
    buffer.record(0xCAFEBABE, false);

    // Merge into a trie (should not panic)
    let mut trie = LinearIntentTrie::new(64);
    buffer.merge_into(&mut trie);

    // Clear should succeed
    buffer.clear();

    // After clear, merge should be a no-op
    buffer.merge_into(&mut trie);

    let overhead = t.elapsed();
    println!("test_reconciliation_buffer_record_and_clear: Testing Overhead = {:?}", overhead);
}

/// Verifies that merge can handle a large number of entries.
#[test]
fn test_reconciliation_buffer_stress() {
    let t = Instant::now();

    let mut buffer = ReconciliationBuffer::new();

    // Record 10K learning events
    for i in 0..10_000u64 {
        buffer.record(i, i % 2 == 0);
    }

    // Merge should complete without panic
    let mut trie = LinearIntentTrie::new(64);
    buffer.merge_into(&mut trie);

    // Clear
    buffer.clear();

    let overhead = t.elapsed();
    println!("test_reconciliation_buffer_stress: Testing Overhead = {:?}", overhead);
}
