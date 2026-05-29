use std::collections::HashMap;
use std::time::Instant;

/// Tracks upstream failures observed during request proxying.
///
/// If an upstream exceeds the failure threshold it is ejected from
/// the pool for a cooldown period.
pub struct PassiveHealth {
    failures: HashMap<String, u32>,
    ejected: HashMap<String, Instant>,
}

impl PassiveHealth {
    pub fn new() -> Self {
        Self {
            failures: HashMap::new(),
            ejected: HashMap::new(),
        }
    }

    /// Record a failed request to an upstream.
    pub fn record_failure(&mut self, upstream: &str, max_fails: u32, fail_timeout: std::time::Duration) {
        let _ = (upstream, max_fails, fail_timeout);
        todo!("increment failure count, eject if threshold exceeded")
    }

    /// Record a successful request to an upstream.
    pub fn record_success(&mut self, upstream: &str) {
        let _ = upstream;
        todo!("reset failure count for upstream")
    }

    /// Check whether an upstream is currently ejected.
    pub fn is_ejected(&self, upstream: &str, _now: Instant) -> bool {
        let _ = upstream;
        todo!("check if upstream is in ejected map and cooldown hasn't expired")
    }
}

impl Default for PassiveHealth {
    fn default() -> Self {
        Self::new()
    }
}
