use std::sync::atomic::{AtomicU64, Ordering};

use super::{LoadBalancer, RequestContext, UpstreamPool};

/// Plain round-robin load balancer (all upstreams treated equally).
///
/// Increments an atomic counter on each call and picks
/// `counter % healthy_count`. Unhealthy upstreams are skipped.
pub struct RoundRobin {
    counter: AtomicU64,
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancer for RoundRobin {
    fn select(&self, pool: &UpstreamPool, _ctx: &RequestContext) -> Option<usize> {
        let healthy: Vec<usize> = pool
            .upstreams
            .iter()
            .enumerate()
            .filter(|(_, u)| u.healthy.load(Ordering::Relaxed))
            .map(|(i, _)| i)
            .collect();

        if healthy.is_empty() {
            return None;
        }

        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        let idx = (count as usize) % healthy.len();
        Some(healthy[idx])
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn make_pool(n: usize) -> UpstreamPool {
        let upstreams: Vec<Arc<super::super::UpstreamState>> = (0..n)
            .map(|i| Arc::new(super::super::UpstreamState::new(format!("10.0.0.{}:3000", i + 1), 1)))
            .collect();

        UpstreamPool {
            name: "test".into(),
            strategy: crate::config::upstream::Strategy::RoundRobin,
            upstreams,
            session_config: None,
            balancer: Arc::new(RoundRobin::new()),
        }
    }

    #[test]
    fn round_robin_cycles_through_all() {
        let pool = make_pool(3);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        let mut seen = vec![];
        for _ in 0..6 {
            let (addr, _guard) = pool.acquire(&ctx).unwrap();
            seen.push(addr);
        }

        assert_eq!(seen[0], "10.0.0.1:3000");
        assert_eq!(seen[1], "10.0.0.2:3000");
        assert_eq!(seen[2], "10.0.0.3:3000");
        assert_eq!(seen[3], "10.0.0.1:3000");
        assert_eq!(seen[4], "10.0.0.2:3000");
        assert_eq!(seen[5], "10.0.0.3:3000");
    }

    #[test]
    fn round_robin_skips_unhealthy() {
        let pool = make_pool(3);
        pool.upstreams[1].healthy.store(false, Ordering::Relaxed);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        for _ in 0..4 {
            let (addr, _guard) = pool.acquire(&ctx).unwrap();
            assert_ne!(addr, "10.0.0.2:3000");
        }
    }

    #[test]
    fn round_robin_all_unhealthy() {
        let pool = make_pool(2);
        pool.upstreams[0].healthy.store(false, Ordering::Relaxed);
        pool.upstreams[1].healthy.store(false, Ordering::Relaxed);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        assert!(pool.acquire(&ctx).is_none());
    }
}
