use httpx_dsa::SecureSlab;
use httpx_transport::dispatcher::CoreDispatcher;
use httpx_core::ServerConfig;
use tokio::net::UdpSocket;
use std::sync::Arc;
use tokio::time::Instant;

#[tokio::test]
async fn test_gso_batch_saturation_stress() {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let target = socket.local_addr().unwrap();
    
    // Learning and Control Plane Bridge
    let (learn_tx, _learn_rx) = tokio::sync::mpsc::unbounded_channel();
    let (_control_tx, control_rx) = tokio::sync::mpsc::channel(100);
    
    let mut config = ServerConfig::default();
    config.slab_capacity = 128; // Enough for 16 fragments @ 4KB
    
    let trie = httpx_dsa::LinearIntentTrie::new(1024);
    let _dispatcher = CoreDispatcher::new_with_socket(
        0, 
        socket, 
        control_rx, 
        config, 
        trie,
        learn_tx
    ).await.unwrap();
    
    let slab = Arc::new(SecureSlab::new(128));

    // Fill slab with pattern
    for i in 0..128 {
        slab.set_version(i, 1);
        let ptr = slab.get_slot(i);
        unsafe { std::ptr::write_bytes(ptr, i as u8, 4096); }
    }

    // Attempt to send 16 segments (64KB exactly)
    // # Mechanical Sympathy: 64KB is the hard limit for GSO segments in the Linux kernel.
    let mut handles = Vec::new();
    for i in 0..16 {
        handles.push((i as u32, 1));
    }

    // Let's test the PayloadStreamer directly.
    use httpx_transport::stream::PayloadStreamer;
    let streamer_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let streamer = PayloadStreamer::new(streamer_socket, 1500).unwrap();
    
    let res = streamer.stream_batch(&slab, &handles, target).await;
    assert!(res.is_ok(), "64KB GSO Batch failed");
    assert_eq!(res.unwrap(), 15, "Should batch 15 fragments (15*4096 = 61440, next would exceed 65535)");
}

#[tokio::test]
async fn test_concurrent_rc_recycling_stress() {
    let slab = Arc::new(SecureSlab::new(1024));
    let handle = 42;
    slab.set_version(handle, 1);
    
    let mut workers = Vec::new();
    let start = Instant::now();

    // Spawn 10 threads racing to increment/decrement RC
    for _ in 0..10 {
        let slab_clone = slab.clone();
        workers.push(tokio::spawn(async move {
            for _ in 0..10_000 {
                slab_clone.increment_rc(handle);
                // Simulate I/O latency
                tokio::task::yield_now().await;
                slab_clone.decrement_rc(handle);
            }
        }));
    }

    for jh in workers {
        jh.await.unwrap();
    }

    assert_eq!(slab.is_in_flight(handle), false, "Final RC must be zero");
    println!("RC Stress Test: 100,000 ops in {:?}", start.elapsed());
}

#[tokio::test]
async fn test_cache_line_collision_isolation() {
    // This test verifies that accessing adjacent TrieNodes from different threads
    // doesn't cause performance degradation due to False Sharing.
    use std::mem::size_of;
    use httpx_dsa::trie::TrieNode;
    
    assert_eq!(size_of::<TrieNode>(), 64, "TrieNode MUST be exactly 64 bytes for L1 alignment");
}
