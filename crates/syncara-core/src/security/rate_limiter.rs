use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sliding-window rate limiter keyed by client IP.
///
/// Each IP gets a time-ordered vector of request timestamps.  Entries
/// older than the window are trimmed on each check.
///
/// Bounded: when the map exceeds `max_buckets`, stale entries (with
/// empty timestamp vecs) are evicted to prevent unbounded memory growth
/// under IP rotation attacks.
pub struct RateLimiter {
    max_requests: usize,
    window: Duration,
    max_buckets: usize,
    buckets: Mutex<HashMap<IpAddr, Vec<Instant>>>,

    /// Violation count per IP for auto-blocklist integration.
    /// Reset when no recent violations.
    violations: Mutex<HashMap<IpAddr, (u32, Instant)>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            max_buckets: 100_000,
            buckets: Mutex::new(HashMap::new()),
            violations: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_max_buckets(mut self, max: usize) -> Self {
        self.max_buckets = max;
        self
    }

    pub fn check(&self, client_ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter lock");
        let timestamps = buckets.entry(client_ip).or_default();

        let cutoff = now - self.window;
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.max_requests {
            // Record violation for auto-block tracking
            if let Ok(mut v) = self.violations.lock() {
                v.entry(client_ip)
                    .and_modify(|(count, last)| {
                        *count += 1;
                        *last = now;
                    })
                    .or_insert((1, now));
            }
            return false;
        }

        timestamps.push(now);

        // Opportunistic eviction when map is large.
        // Evicts the entry with the oldest recent timestamp to bound
        // memory under IP rotation attacks.
        if buckets.len() > self.max_buckets {
            if let Some(oldest_ip) = buckets
                .iter()
                .filter(|(_, ts)| !ts.is_empty())
                .min_by_key(|(_, ts)| ts[ts.len() - 1])
                .map(|(ip, _)| *ip)
            {
                buckets.remove(&oldest_ip);
            }
        }

        true
    }

    /// Return the violation count for `client_ip`, resetting if the
    /// last violation was outside the rate-limit window.
    pub fn violation_count(&self, client_ip: IpAddr) -> u32 {
        let now = Instant::now();
        if let Ok(mut v) = self.violations.lock() {
            if let Some((count, last)) = v.get(&client_ip) {
                if now.duration_since(*last) <= self.window {
                    return *count;
                }
                // Stale violation record — remove
                v.remove(&client_ip);
            }
        }
        0
    }

    /// Clear a specific IP's violation count (called when auto-block
    /// is triggered so we don't double-count).
    pub fn clear_violations(&self, client_ip: IpAddr) {
        if let Ok(mut v) = self.violations.lock() {
            v.remove(&client_ip);
        }
    }

    pub fn len(&self) -> usize {
        self.buckets.lock().expect("rate limiter lock").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn allows_within_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        for _ in 0..5 {
            assert!(limiter.check("10.0.0.1".parse().unwrap()));
        }
    }

    #[test]
    fn rejects_above_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        for _ in 0..3 {
            assert!(limiter.check("10.0.0.1".parse().unwrap()));
        }
        assert!(!limiter.check("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn different_ips_independent() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.check("10.0.0.1".parse().unwrap()));
        assert!(limiter.check("10.0.0.1".parse().unwrap()));
        assert!(!limiter.check("10.0.0.1".parse().unwrap()));

        assert!(limiter.check("10.0.0.2".parse().unwrap()));
        assert!(limiter.check("10.0.0.2".parse().unwrap()));
    }

    #[test]
    fn window_slides() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        assert!(limiter.check("10.0.0.1".parse().unwrap()));
        assert!(limiter.check("10.0.0.1".parse().unwrap()));
        assert!(!limiter.check("10.0.0.1".parse().unwrap()));

        thread::sleep(Duration::from_millis(60));
        assert!(limiter.check("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn eviction_bounds_map_size() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60))
            .with_max_buckets(2);
        let ip1: IpAddr = "10.0.0.1".parse().unwrap();
        let ip2: IpAddr = "10.0.0.2".parse().unwrap();
        let ip3: IpAddr = "10.0.0.3".parse().unwrap();

        assert!(limiter.check(ip1));
        assert!(limiter.check(ip2));
        assert_eq!(limiter.len(), 2);
        // Adding ip3 triggers eviction of ip1 (oldest).
        assert!(limiter.check(ip3));
        assert_eq!(limiter.len(), 2); // ip2 + ip3, ip1 evicted
    }

    #[test]
    fn violation_count_tracks_rejections() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip));
        assert_eq!(limiter.violation_count(ip), 1);
        assert!(!limiter.check(ip));
        assert_eq!(limiter.violation_count(ip), 2);
    }
}
