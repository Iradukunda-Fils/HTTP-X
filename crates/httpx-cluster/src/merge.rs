use crate::gossip::IntentDelta;
use httpx_core::PredictiveEngine;
use httpx_dsa::LinearIntentTrie;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Background worker that accumulates weight deltas and performs the Shadow-Swap.
pub struct WeightAggregator {
    engine: Arc<PredictiveEngine>,
    delta_rx: mpsc::Receiver<IntentDelta>,
    shadow_trie: LinearIntentTrie,
    /// Counter for "Significant Shift" detection.
    total_delta: u64,
}

impl WeightAggregator {
    pub fn new(engine: Arc<PredictiveEngine>, delta_rx: mpsc::Receiver<IntentDelta>) -> Self {
        Self {
            engine,
            delta_rx,
            shadow_trie: LinearIntentTrie::new(1024),
            total_delta: 0,
        }
    }

    /// Background loop for aggregation and periodic swapping.
    pub async fn run_loop(&mut self) {
        let mut timer = interval(Duration::from_millis(100));
        
        loop {
            tokio::select! {
                Some(delta) = self.delta_rx.recv() => {
                    self.apply_delta(delta);
                }
                _ = timer.tick() => {
                    self.trigger_swap();
                }
            }
        }
    }

    fn apply_delta(&mut self, delta: IntentDelta) {
        // # Mechanical Sympathy: In a real implementation, we'd map the hash
        // to a specific trie path. Here we simulate the weight update.
        // For simplicity, we use the hash as a node index (not for production).
        
        // Accumulate deltas (Fixed-Point to Markov weight conversion)
        self.total_delta += (delta.delta_true + delta.delta_false) as u64;
        
        // Logic for "Significant Shift"
        if self.total_delta > 1000 {
            self.trigger_swap();
        }
    }

    fn trigger_swap(&mut self) {
        if self.total_delta == 0 { return; }
        
        tracing::info!("WeightAggregator: Triggering Shadow-Swap (Delta: {})", self.total_delta);
        
        // Clone the shadow trie to perform an atomic update to the engine
        let trie_to_swap = self.shadow_trie.clone();
        self.engine.swap_weights(trie_to_swap);
        
        // Reset shift counter
        self.total_delta = 0;
    }
}
