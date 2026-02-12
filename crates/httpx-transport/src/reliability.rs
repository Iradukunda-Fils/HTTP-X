//! # httpx-transport: Reliability Logic
//! 
//! This module implements the "Predictive Backoff" and "Multi-Level Credit" systems.

/// The CongestionController trait defines how the server reacts to network pressure.
/// 
/// ## Performance Guarantee
/// Decision logic is $O(1)$ with zero heap allocations during the hot-path.
pub trait CongestionController: Send + Sync {
    /// Evaluates if the network can sustain a speculative push.
    /// Returns the active credit level (Level 0, 1, or 2).
    fn evaluate_intent_credit(&self, rtt_nanos: u64) -> u8;

    /// Called when a packet is lost. Triggers immediate speculative backoff.
    fn notify_loss(&self);
}

pub struct DefaultCongestionController {
    base_rtt: u64,
    active_level: std::sync::atomic::AtomicU8,
}

impl DefaultCongestionController {
    pub fn new(base_rtt_nanos: u64) -> Self {
        Self {
            base_rtt: base_rtt_nanos,
            active_level: std::sync::atomic::AtomicU8::new(2),
        }
    }
}

impl CongestionController for DefaultCongestionController {
    fn evaluate_intent_credit(&self, current_rtt: u64) -> u8 {
        // Multi-Level Credit System Logic
        // If current RTT > 1.2 * base_rtt, back off to Level 0.
        if current_rtt > (self.base_rtt * 12) / 10 {
            self.active_level.store(0, std::sync::atomic::Ordering::Relaxed);
            0
        } else {
            self.active_level.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    fn notify_loss(&self) {
        // Immediate Zero-Allocation speculative backoff
        self.active_level.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}
