use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterMode {
    Integrated,
    Sovereign,
}

/// A Hysteresis-aware Monitor for Cluster Stability.
/// 
/// Uses a Leaky Bucket approach to prevent "Mode Jitter" during 
/// network instability (flapping).
pub struct ClusterStability {
    mode: ClusterMode,
    consecutive_misses: u32,
    consecutive_stable: u32,
    last_pulse: Instant,
    miss_threshold: u32,
    recovery_threshold: u32,
}

impl ClusterStability {
    pub fn new() -> Self {
        Self {
            mode: ClusterMode::Integrated,
            consecutive_misses: 0,
            consecutive_stable: 0,
            last_pulse: Instant::now(),
            miss_threshold: 3,    // Panic after 3 missed pulses
            recovery_threshold: 10, // Recover after 10 stable pulses
        }
    }

    /// Records a successful gossip heartbeat.
    pub fn record_success(&mut self) {
        self.consecutive_misses = 0;
        self.last_pulse = Instant::now();
        
        if self.mode == ClusterMode::Sovereign {
            self.consecutive_stable += 1;
            if self.consecutive_stable >= self.recovery_threshold {
                self.transition(ClusterMode::Integrated);
            }
        }
    }

    /// Records a missed gossip heartbeat or timeout.
    pub fn record_miss(&mut self) {
        self.consecutive_stable = 0;
        self.consecutive_misses += 1;
        
        if self.mode == ClusterMode::Integrated {
            if self.consecutive_misses >= self.miss_threshold {
                self.transition(ClusterMode::Sovereign);
            }
        }
    }

    pub fn current_mode(&self) -> ClusterMode {
        self.mode
    }

    fn transition(&mut self, new_mode: ClusterMode) {
        let guard = crossbeam_epoch::pin();
        tracing::warn!(
            "HYSTERESIS: Transitioning from {:?} to {:?} [Epoch: {:?}]", 
            self.mode, 
            new_mode,
            guard.collector() // Simulated Epoch ID for debugging global state timeline
        );
        self.mode = new_mode;
        self.consecutive_misses = 0;
        self.consecutive_stable = 0;
    }
}
