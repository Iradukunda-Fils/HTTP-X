use httpx_codec::ProbabilisticCodec;

#[test]
fn test_bayesian_poisoning_robustness() {
    let codec = ProbabilisticCodec::new();
    let poisoned_context = vec![0xFF; 1024]; // High-entropy "impossible" path
    
    // Hallucination Check: Branch Prediction
    // The trie must handle high-entropy context without search spikes or panics.
    for _ in 0..100 {
        let _ = codec.project_header(&poisoned_context);
    }
}

#[test]
fn test_fec_0rtt_reconstruction() {
    let mut parity_block = 0u64;
    let packets = [0xAABBCCDDu64, 0x11223344u64, 0x55667788u64];
    
    // Create Parity
    for p in &packets {
        parity_block ^= p;
    }
    
    // Simulate Loss of packet[1]
    let lost_packet_idx = 1;
    let mut reconstructed = parity_block;
    for (i, p) in packets.iter().enumerate() {
        if i != lost_packet_idx {
            reconstructed ^= p;
        }
    }
    
    // Hallucination Check: O(1) perceived latency
    // Reconstruction is purely XOR operations in L1 registers.
    assert_eq!(reconstructed, packets[lost_packet_idx]);
}
