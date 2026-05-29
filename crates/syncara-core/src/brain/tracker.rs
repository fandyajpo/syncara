use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Global latency tracker, initialised once at startup.
static TRACKER: OnceLock<LatencyTracker> = OnceLock::new();

pub fn init() -> &'static LatencyTracker {
    TRACKER.get_or_init(|| LatencyTracker::new(100))
}

pub fn get() -> &'static LatencyTracker {
    TRACKER
        .get()
        .expect("LatencyTracker not initialised — call brain::tracker::init() first")
}

/// Per-upstream latency ring buffer.
///
/// Stores the most recent N observed response latencies per upstream
/// and exposes p50 / global-p50 queries for the Brain scoring engine.
pub struct LatencyTracker {
    ring_size: usize,
    buf: Mutex<HashMap<String, Vec<f64>>>,
}

impl LatencyTracker {
    fn new(ring_size: usize) -> Self {
        Self {
            ring_size,
            buf: Mutex::new(HashMap::new()),
        }
    }

    /// Record a single latency observation for `addr`.
    pub fn record(&self, addr: &str, latency_s: f64) {
        let mut buf = self.buf.lock().expect("tracker lock");
        let ring = buf.entry(addr.to_string()).or_insert_with(|| {
            Vec::with_capacity(self.ring_size)
        });
        if ring.len() >= self.ring_size {
            ring.remove(0);
        }
        ring.push(latency_s);
    }

    /// Median (p50) latency for a single upstream, in seconds.
    pub fn p50(&self, addr: &str) -> Option<f64> {
        let buf = self.buf.lock().expect("tracker lock");
        let ring = buf.get(addr)?;
        if ring.is_empty() {
            return None;
        }
        let mut sorted = ring.clone();
        sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(sorted[sorted.len() / 2])
    }

    /// Median (p50) latency across all observed upstreams.
    pub fn global_p50(&self) -> Option<f64> {
        let buf = self.buf.lock().expect("tracker lock");
        let mut all: Vec<f64> = buf.values().flat_map(|v| v.iter().copied()).collect();
        if all.is_empty() {
            return None;
        }
        all.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(all[all.len() / 2])
    }

    pub fn clear(&self) {
        let mut buf = self.buf.lock().expect("tracker lock");
        buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_p50() {
        let t = LatencyTracker::new(5);
        assert!(t.p50("10.0.0.1:3000").is_none());

        t.record("10.0.0.1:3000", 0.010);
        t.record("10.0.0.1:3000", 0.020);
        t.record("10.0.0.1:3000", 0.030);

        let p = t.p50("10.0.0.1:3000").unwrap();
        assert!((p - 0.020).abs() < 1e-9);
    }

    #[test]
    fn ring_buffer_caps() {
        let t = LatencyTracker::new(3);
        for i in 0..10 {
            t.record("10.0.0.1:3000", i as f64 * 0.010);
        }
        // Only the last 3 remain: 0.070, 0.080, 0.090 → p50 = 0.080
        let p = t.p50("10.0.0.1:3000").unwrap();
        assert!((p - 0.080).abs() < 1e-9);
    }

    #[test]
    fn global_p50_multi_upstream() {
        let t = LatencyTracker::new(10);
        t.record("a:3000", 0.010);
        t.record("a:3000", 0.020);
        t.record("b:3000", 0.100);
        t.record("b:3000", 0.200);

        // Sorted: 0.010, 0.020, 0.100, 0.200 → p50 = 0.100 (index 2)
        let gp = t.global_p50().unwrap();
        assert!((gp - 0.100).abs() < 1e-9);
    }

    #[test]
    fn global_p50_empty() {
        let t = LatencyTracker::new(5);
        assert!(t.global_p50().is_none());
    }

    #[test]
    fn clear_resets() {
        let t = LatencyTracker::new(5);
        t.record("10.0.0.1:3000", 0.010);
        assert!(t.p50("10.0.0.1:3000").is_some());
        t.clear();
        assert!(t.p50("10.0.0.1:3000").is_none());
    }
}
