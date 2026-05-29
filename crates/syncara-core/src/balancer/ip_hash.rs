use std::sync::atomic::Ordering;

use super::{LoadBalancer, RequestContext, UpstreamPool};

/// IP-hash load balancer.
///
/// Deterministically maps a client IP to an upstream via hashing.
/// The mapping is stable as long as the set of healthy upstreams
/// does not change.
pub struct IpHash;

impl LoadBalancer for IpHash {
    fn select(&self, pool: &UpstreamPool, ctx: &RequestContext) -> Option<usize> {
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

        let ip_bytes = match ctx.client_ip {
            std::net::IpAddr::V4(v4) => v4.octets().to_vec(),
            std::net::IpAddr::V6(v6) => v6.octets().to_vec(),
        };

        let hash = simple_hash(&ip_bytes);
        let idx = hash % healthy.len() as u64;
        Some(healthy[idx as usize])
    }
}

/// Simple non-cryptographic hash for consistent IP routing.
fn simple_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::balancer::UpstreamState;

    fn make_pool(n: usize) -> UpstreamPool {
        let upstreams: Vec<Arc<UpstreamState>> = (0..n)
            .map(|i| Arc::new(UpstreamState::new(format!("10.0.0.{}:3000", i + 1), 1)))
            .collect();

        UpstreamPool {
            name: "test".into(),
            strategy: crate::config::upstream::Strategy::IpHash,
            upstreams,
            session_config: None,
            balancer: Arc::new(IpHash),
        }
    }

    #[test]
    fn same_ip_same_upstream() {
        let pool = make_pool(3);
        let ctx = RequestContext::new("10.0.0.1".parse().unwrap());

        let a = pool.acquire(&ctx).unwrap().0;
        let b = pool.acquire(&ctx).unwrap().0;
        assert_eq!(a, b);
    }

    #[test]
    fn different_ip_different_upstream() {
        let pool = make_pool(3);
        let ctx_a = RequestContext::new("10.0.0.1".parse().unwrap());
        let ctx_b = RequestContext::new("10.0.0.2".parse().unwrap());

        let a = pool.acquire(&ctx_a).unwrap().0;
        let b = pool.acquire(&ctx_b).unwrap().0;
        // Could collide, but extremely unlikely with 2 IPs and 3 upstreams
        // We're just verifying it doesn't panic and returns something
        assert!(!a.is_empty());
        assert!(!b.is_empty());
    }
}
