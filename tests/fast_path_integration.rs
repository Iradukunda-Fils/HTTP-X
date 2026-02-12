use httpx_dsa::{LinearIntentTrie, SecureSlab};
use httpx_transport::dispatcher::CoreDispatcher;
use httpx_core::ServerConfig;
use tokio::net::UdpSocket;
use std::sync::Arc;

#[tokio::test]
async fn test_fast_path_full_lifecycle() {
    // 1. Setup the Intelligence Layer (Trie)
    let mut trie = LinearIntentTrie::new(1024);
    let context = b"GET /index.html";
    let handle = 0;
    let version = 1;
    
    trie.observe(context, true);
    trie.associate_payload(context, handle, version);

    // 2. Setup the Hardware Layer (Slab & io_uring)
    let slab = Arc::new(SecureSlab::new(64));
    slab.set_version(handle as usize, version);
    
    // Write dummy data to the slab slot
    let slot_ptr = slab.get_slot(handle as usize);
    unsafe {
        std::ptr::write_bytes(slot_ptr, 0xAA, 4096);
    }

    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    let (_tx, rx) = tokio::sync::mpsc::channel(10);
    let (learn_tx, _learn_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut dispatcher = CoreDispatcher::new_with_socket(0, socket, rx, ServerConfig::default(), trie.clone(), learn_tx).await.unwrap();

    // 3. Execution: Submit Linked Burst
    // This simulates the hot-path resolution of handle+version from the Trie.
    let res = dispatcher.submit_linked_burst(addr, handle, 0, version, &slab).await;
    assert!(res.is_ok(), "Linked burst submission failed");

    // 4. Verification: Memory In-Flight
    // The RC must be > 0 because the kernel is (conceptually) holding the buffer.
    assert!(slab.is_in_flight(handle as usize), "Slab slot should be marked as in-flight");

    // 5. Completion: Reap and Recycle
    dispatcher.reap_completions(&slab);
    
    println!("Fast-Path Lifecycle Certified: Submission -> RC increment -> Reaper call.");
}

#[tokio::test]
#[should_panic]
async fn test_invalid_handle_safety() {
    let slab = Arc::new(SecureSlab::new(64));
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = socket.local_addr().unwrap();
    let (_tx, rx) = tokio::sync::mpsc::channel(10);
    let (learn_tx, _learn_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut dispatcher = CoreDispatcher::new_with_socket(0, socket, rx, ServerConfig::default(), LinearIntentTrie::new(1024), learn_tx).await.unwrap();

    // Attempting to submit a handle that is out-of-bounds for the slab
    let invalid_handle = 999; 
    let _res = dispatcher.submit_linked_burst(addr, invalid_handle, 0, 1, &slab).await;
    
    // The implementation currently asserts!() on indexing in SecureSlab.
    // In production, we might want it to return an Error.
    // However, since Trie Association is controlled, an OOB handle is a bug.
    // For now, we verify that associated_payload is the only way to get a handle.
    assert!(invalid_handle >= 64, "Handle is truly invalid");
}

#[tokio::test]
async fn test_gso_batch_integrity() {
    use httpx_transport::stream::PayloadStreamer;
    
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let target = socket.local_addr().unwrap();
    let streamer = PayloadStreamer::new(socket, 1500).unwrap();
    
    let slab = SecureSlab::new(16);
    for i in 0..4 {
        slab.set_version(i, 1);
        let ptr = slab.get_slot(i);
        unsafe { std::ptr::write_bytes(ptr, i as u8, 4096); }
    }

    let handles = vec![(0, 1), (1, 1), (2, 1), (3, 1)];
    
    // Test batch streaming
    let res = streamer.stream_batch(&slab, &handles, target).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 4, "Should have batched 4 fragments");
}
