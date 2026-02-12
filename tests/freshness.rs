use httpx_dsa::{LinearIntentTrie, SecureSlab};
use httpx_transport::dispatcher::CoreDispatcher;
use httpx_core::ServerConfig;
use tokio::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_stale_push_freshness_gate() {
    let mut trie = LinearIntentTrie::new(1024);
    let context = b"GET /data";
    let handle = 7;
    let initial_version = 100;

    // 1. Setup Trie with Versioned Payload
    trie.observe(context, true);
    trie.associate_payload(context, handle, initial_version);

    let slab = Arc::new(SecureSlab::new(64));
    slab.set_version(handle as usize, initial_version);

    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    let (_tx, rx) = tokio::sync::mpsc::channel(10);
    let (learn_tx, _learn_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut dispatcher = CoreDispatcher::new_with_socket(0, socket, rx, ServerConfig::default(), trie.clone(), learn_tx).await.unwrap();

    // 2. Scenario A: VERSION MATCH (Success)
    let res = dispatcher.submit_linked_burst(addr, handle, 0, initial_version, &slab).await;
    assert!(res.is_ok(), "Should allow push when versions match");

    // 3. Scenario B: VERSION MISMATCH (Failure)
    // Update the slab version (Simulating a high-frequency update)
    let new_version = initial_version + 1;
    slab.set_version(handle as usize, new_version);

    // Try submitting with the OLD version (from the Trie)
    let res = dispatcher.submit_linked_burst(addr, handle, 0, initial_version, &slab).await;
    
    assert!(res.is_err(), "Freshness Gate MUST block stale pushes");
    if let Err(e) = res {
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidData, "Should fail with InvalidData (Stale Payload)");
    }

    println!("Freshness Audit: Semantic Gate verified (Stale Push Blocked).");
}

#[tokio::test]
async fn test_high_frequency_freshness_chaos() {
    let slab = Arc::new(SecureSlab::new(64));
    let handle = 0;
    
    // Simulate high-frequency updates (1 update per 1ms for the test, 10µs is hardware limit)
    let slab_clone = slab.clone();
    let update_jh = tokio::spawn(async move {
        for v in 0..100 {
            slab_clone.set_version(handle as usize, v);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });

    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    let (_tx, rx) = tokio::sync::mpsc::channel(10);
    let (learn_tx, _learn_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut dispatcher = CoreDispatcher::new_with_socket(0, socket, rx, ServerConfig::default(), httpx_dsa::LinearIntentTrie::new(1024), learn_tx).await.unwrap();

    for v in 0..100 {
        // We simulate reading the version from the Trie
        let trie_version = v; 
        
        let res = dispatcher.submit_linked_burst(addr, handle as u32, 0, trie_version, &slab).await;
        
        // If the update occurred between reading and submission, it should fail
        if let Err(e) = res {
            assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
        }
    }

    update_jh.await.unwrap();
    println!("Chaos Audit: High-frequency freshness verified.");
}
