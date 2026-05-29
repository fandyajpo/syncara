use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::{LoadBalancer, RequestContext, UpstreamPool, UpstreamState};

/// Weighted round-robin load balancer.
///
/// Each upstream receives traffic proportional to its weight.
/// Internally builds a virtual ring where each upstream appears
/// `weight` times (reduced by GCD) and advances an atomic index
/// through the ring.
pub struct WeightedRoundRobin {
    /// Virtual ring of upstream indices.
    ring: Vec<usize>,
    /// Ring length.
    len: usize,
    /// Atomic index into the ring.
    index: AtomicU64,
}

impl WeightedRoundRobin {
    pub fn new(upstreams: &[Arc<UpstreamState>]) -> Self {
        let weights: Vec<u32> = upstreams.iter().map(|u| u.weight).collect();
        let gcd = Self::gcd_of(&weights);
        let reduced: Vec<u32> = weights.iter().map(|w| w / gcd).collect();

        let mut ring = Vec::new();
        for (i, &w) in reduced.iter().enumerate() {
            for _ in 0..w {
                ring.push(i);
            }
        }

        // If all weights are 0, fall back to single entry per upstream
        let ring = if ring.is_empty() {
            (0..upstreams.len()).collect()
        } else {
            ring
        };

        let len = ring.len();
        Self {
            ring,
            len,
            index: AtomicU64::new(0),
        }
    }

    fn gcd_of(values: &[u32]) -> u32 {
        values.iter().copied().reduce(Self::gcd).unwrap_or(1)
    }

    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }
}

impl LoadBalancer for WeightedRoundRobin {
    fn select(&self, pool: &UpstreamPool, _ctx: &RequestContext) -> Option<usize> {
        if self.len == 0 {
            return None;
        }

        // Try up to ring.len() times to find a healthy upstream
        for _ in 0..self.len {
            let pos = self.index.fetch_add(1, Ordering::Relaxed) as usize % self.len;
            let idx = self.ring[pos];
            if pool.upstreams[idx].healthy.load(Ordering::Relaxed) {
                return Some(idx);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use super::*;
    use crate::balancer::RequestContext;

    fn make_state(addr: &str, weight: u32) -> Arc<UpstreamState> {
        Arc::new(UpstreamState::new(addr.to_string(), weight))
    }

    fn make_pool(states: Vec<Arc<UpstreamState>>) -> UpstreamPool {
        let upstreams = states;
        let balancer = Arc::new(WeightedRoundRobin::new(&upstreams)) as Arc<dyn LoadBalancer>;

        UpstreamPool {
            name: "test".into(),
            strategy: crate::config::upstream::Strategy::Weighted,
            upstreams,
            session_config: None,
            balancer,
        }
    }

    #[test]
    fn equal_weights_round_robin() {
        let pool = make_pool(vec![
            make_state("10.0.0.1:3000", 1),
            make_state("10.0.0.2:3000", 1),
            make_state("10.0.0.3:3000", 1),
        ]);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        let mut seen = vec![];
        for _ in 0..6 {
            let (addr, _g) = pool.acquire(&ctx).unwrap();
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
    fn weighted_distribution() {
        let pool = make_pool(vec![
            make_state("10.0.0.1:3000", 3),
            make_state("10.0.0.2:3000", 1),
        ]);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        let mut seen = vec![];
        for _ in 0..8 {
            let (addr, _g) = pool.acquire(&ctx).unwrap();
            seen.push(addr);
        }

        // Ring: [0, 0, 0, 1] (3:1 ratio after GCD)
        let count_0 = seen.iter().filter(|a| *a == "10.0.0.1:3000").count();
        let count_1 = seen.iter().filter(|a| *a == "10.0.0.2:3000").count();
        assert_eq!(count_0, 6); // 3/4 of 8
        assert_eq!(count_1, 2); // 1/4 of 8
    }

    #[test]
    fn weight_unequal_gcd() {
        let pool = make_pool(vec![
            make_state("10.0.0.1:3000", 6),
            make_state("10.0.0.2:3000", 4),
        ]);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        // GCD is 2 → reduced to [3, 2], ring length 5
        let mut seen = vec![];
        for _ in 0..5 {
            let (addr, _g) = pool.acquire(&ctx).unwrap();
            seen.push(addr);
        }

        let count_0 = seen.iter().filter(|a| *a == "10.0.0.1:3000").count();
        let count_1 = seen.iter().filter(|a| *a == "10.0.0.2:3000").count();
        assert_eq!(count_0, 3);
        assert_eq!(count_1, 2);
    }

    #[test]
    fn weighted_skips_unhealthy() {
        let pool = make_pool(vec![
            make_state("10.0.0.1:3000", 1),
            make_state("10.0.0.2:3000", 1),
        ]);
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        pool.upstreams[0].healthy.store(false, Ordering::Relaxed);

        for _ in 0..4 {
            let (addr, _g) = pool.acquire(&ctx).unwrap();
            assert_eq!(addr, "10.0.0.2:3000");
        }
    }
}
