use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    ClusterIntegrated,
    SovereignAutonomous,
}

pub struct Session {
    pub addr: SocketAddr,
    pub mode: SessionMode,
    /// Initial Intent Window (IIW) credits.
    /// Decremented on each predictive push, replenished on IntentAck.
    pub iiw_credit: AtomicUsize,
    /// Priority-Zero Pivot: If true, all predictive pushes are blocked.
    pub canceled: AtomicBool,
}

impl Session {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            mode: SessionMode::ClusterIntegrated,
            iiw_credit: AtomicUsize::new(10), // Start with foundational 10 credits
            canceled: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
    }

    pub fn reset_pivot(&self) {
        self.canceled.store(false, Ordering::Release);
    }

    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::Acquire)
    }

    /// Replenishes IIW credits upon receiving an IntentAck.
    pub fn replenish_credits(&self) {
        self.iiw_credit.store(10, Ordering::Release);
    }

    /// Consumes one IIW credit for a predictive push.
    /// Returns `true` if a credit was available.
    pub fn consume_credit(&self) -> bool {
        loop {
            let current = self.iiw_credit.load(Ordering::Acquire);
            if current == 0 {
                return false;
            }
            if self.iiw_credit.compare_exchange(
                current, 
                current - 1, 
                Ordering::AcqRel, 
                Ordering::Acquire
            ).is_ok() {
                return true;
            }
        }
    }

    /// Check if credit is available without consuming.
    pub fn has_credit(&self) -> bool {
        self.iiw_credit.load(Ordering::Acquire) > 0
    }
}
