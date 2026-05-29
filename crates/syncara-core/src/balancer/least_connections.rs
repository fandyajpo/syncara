use std::sync::atomic::Ordering;

use super::{LoadBalancer, RequestContext, UpstreamPool};

/// Least-connections load balancer.
///
/// Scans all healthy upstreams and picks the one with the fewest
/// active connections. On ties the first encountered is returned.
pub struct LeastConnections;

impl LoadBalancer for LeastConnections {
    fn select(&self, pool: &UpstreamPool, _ctx: &RequestContext) -> Option<usize> {
        let mut best: Option<(usize, u64)> = None;

        for (i, state) in pool.upstreams.iter().enumerate() {
            if !state.healthy.load(Ordering::Relaxed) {
                continue;
            }
            let conns = state.active_connections.load(Ordering::Relaxed);
            match best {
                None => best = Some((i, conns)),
                Some((_, best_conns)) if conns < best_conns => best = Some((i, conns)),
                _ => {}
            }
        }

        best.map(|(i, _)| i)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use super::*;
    use crate::balancer::{RequestContext, UpstreamPool, UpstreamState};
    use crate::config::upstream::Strategy;

    fn make_pool() -> UpstreamPool {
        let upstreams: Vec<Arc<UpstreamState>> = (0..3)
            .map(|i| Arc::new(UpstreamState::new(format!("10.0.0.{}:3000", i + 1), 1)))
            .collect();

        UpstreamPool {
            name: "test".into(),
            strategy: Strategy::LeastConnections,
            upstreams,
            session_config: None,
            balancer: Arc::new(LeastConnections),
        }
    }

    #[test]
    fn picks_least_connections() {
        let pool = make_pool();
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        // Bump connection count on upstream 0
        pool.upstreams[0]
            .active_connections
            .store(5, Ordering::Relaxed);
        pool.upstreams[1]
            .active_connections
            .store(2, Ordering::Relaxed);
        pool.upstreams[2]
            .active_connections
            .store(10, Ordering::Relaxed);

        let (addr, _guard) = pool.acquire(&ctx).unwrap();
        assert_eq!(addr, "10.0.0.2:3000"); // index 1 has fewest (2)
    }

    #[test]
    fn skips_unhealthy() {
        let pool = make_pool();
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        pool.upstreams[1].healthy.store(false, Ordering::Relaxed);
        let (addr, _guard) = pool.acquire(&ctx).unwrap();
        assert_ne!(addr, "10.0.0.2:3000");
    }

    #[test]
    fn busy_guard_decrements_on_drop() {
        let pool = make_pool();
        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());

        {
            let (_addr, guard) = pool.acquire(&ctx).unwrap();
            assert_eq!(
                pool.upstreams[0]
                    .active_connections
                    .load(Ordering::Relaxed),
                1
            );
            drop(guard);
        }

        assert_eq!(
            pool.upstreams[0]
                .active_connections
                .load(Ordering::Relaxed),
            0
        );
    }
}
