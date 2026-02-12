use serde::{Serialize, Deserialize};
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IntentDelta {
    /// Simplified 64-bit context hash/ID for gossip.
    pub context_hash: u64,
    /// Fixed-Point Weight increments (u16).
    pub delta_true: u16,
    pub delta_false: u16,
    /// Sequence number to prevent stale learning.
    pub sequence_number: u64,
}

/// UDP-based Gossip Protocol for multi-node intent distribution.
pub struct GossipProtocol {
    socket: Arc<UdpSocket>,
    tx_delta: mpsc::Sender<IntentDelta>,
    /// Tracks the highest sequence number seen to date for this node.
    last_seq: std::sync::atomic::AtomicU64,
}

impl GossipProtocol {
    pub fn new(bind_addr: &str, delta_tx: mpsc::Sender<IntentDelta>) -> Self {
        let socket = UdpSocket::bind(bind_addr).expect("Gossip: Failed to bind UDP");
        socket.set_nonblocking(true).expect("Gossip: Failed to set nonblocking");
        
        Self {
            socket: Arc::new(socket),
            tx_delta: delta_tx,
            last_seq: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Broadcasts a weight delta to the cluster.
    pub fn broadcast(&self, peer_addrs: &[String], delta: IntentDelta) {
        let payload = serde_json::to_vec(&delta).unwrap();
        for addr in peer_addrs {
            let _ = self.socket.send_to(&payload, addr);
        }
    }

    /// Background listener for incoming intent deltas.
    pub async fn listen(&self) {
        let mut buf = [0u8; 1024];
        loop {
            if let Ok((len, _)) = self.socket.recv_from(&mut buf) {
                if let Ok(delta) = serde_json::from_slice::<IntentDelta>(&buf[..len]) {
                    // Task 3: Gossip Integrity Proof. Discard stale learning.
                    let current = self.last_seq.load(std::sync::atomic::Ordering::Acquire);
                    if delta.sequence_number > current {
                        if self.last_seq.compare_exchange(
                            current, 
                            delta.sequence_number, 
                            std::sync::atomic::Ordering::AcqRel, 
                            std::sync::atomic::Ordering::Acquire
                        ).is_ok() {
                            let _ = self.tx_delta.send(delta).await;
                        }
                    } else {
                        tracing::warn!("Gossip: Discarding stale update (Seq: {})", delta.sequence_number);
                    }
                }
            }
            tokio::task::yield_now().await;
        }
    }
}
