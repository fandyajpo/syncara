use std::sync::Arc;
use std::time::Duration;

use tracing::debug;

use super::{LoadBalancer, RequestContext, UpstreamPool};
use crate::session::{SessionKey, SessionStore};

/// Sticky session balancer.
///
/// Wraps a fallback balancer (typically round-robin) and a session
/// store. On the first request for a given affinity key the fallback
/// selects an upstream and the binding is persisted. Subsequent
/// requests with the same key are routed to the same upstream as long
/// as it remains healthy.
///
/// When the bound upstream is unhealthy a new one is selected via the
/// fallback and the binding is updated.
pub struct Sticky {
    fallback: Arc<dyn LoadBalancer>,
    store: Arc<dyn SessionStore>,
    ttl: Duration,
}

impl Sticky {
    pub fn new(
        fallback: Arc<dyn LoadBalancer>,
        store: Arc<dyn SessionStore>,
        ttl: Duration,
    ) -> Self {
        Self {
            fallback,
            store,
            ttl,
        }
    }
}

impl LoadBalancer for Sticky {
    fn select(&self, pool: &UpstreamPool, ctx: &RequestContext) -> Option<usize> {
        // Build the session key from the request context.
        let key = match &ctx.sticky_key {
            Some(k) => SessionKey {
                identifier: k.clone(),
            },
            // No sticky key — delegate to fallback (no affinity).
            None => return self.fallback.select(pool, ctx),
        };

        // 1. Look up an existing binding.
        if let Some(addr) = self.store.lookup(&key) {
            if let Some(idx) = pool.upstreams.iter().position(|u| u.addr == addr) {
                if pool.upstreams[idx].healthy.load(std::sync::atomic::Ordering::Relaxed) {
                    debug!(
                        pool = %pool.name,
                        upstream = %addr,
                        key = %key.identifier,
                        "sticky: reused existing binding"
                    );
                    return Some(idx);
                }
                debug!(
                    pool = %pool.name,
                    upstream = %addr,
                    key = %key.identifier,
                    "sticky: bound upstream unhealthy, reselecting"
                );
            } else {
                debug!(
                    pool = %pool.name,
                    upstream = %addr,
                    key = %key.identifier,
                    "sticky: bound upstream no longer in pool, reselecting"
                );
            }
        }

        // 2. First request or stale binding: select via fallback.
        if let Some(idx) = self.fallback.select(pool, ctx) {
            let addr = pool.upstreams[idx].addr.clone();
            self.store.set(key, addr.clone(), self.ttl);
            debug!(
                pool = %pool.name,
                upstream = %addr,
                "sticky: new binding created"
            );
            return Some(idx);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balancer::round_robin::RoundRobin;
    use crate::config::upstream::UpstreamPoolConfig;
    use crate::session::cookie::CookieSession;
    use std::net::IpAddr;
    use std::str::FromStr;

    fn test_pool() -> UpstreamPool {
        let cfg = UpstreamPoolConfig {
            name: "test".into(),
            strategy: crate::config::upstream::Strategy::Sticky,
            upstreams: vec![
                crate::config::upstream::UpstreamConfig {
                    addr: "10.0.0.1:8080".into(),
                    weight: 1,
                    max_connections: None,
                },
                crate::config::upstream::UpstreamConfig {
                    addr: "10.0.0.2:8080".into(),
                    weight: 1,
                    max_connections: None,
                },
            ],
            health: None,
            session: None,
            brain: None,
            connections: None,
        };
        UpstreamPool::from_config(&cfg)
    }

    #[test]
    fn test_sticky_returns_consistent_upstream() {
        let pool = test_pool();
        let store = Arc::new(CookieSession::new("test".into())) as Arc<dyn SessionStore>;
        let balancer = Sticky::new(
            Arc::new(RoundRobin::new()),
            store.clone(),
            Duration::from_secs(60),
        );

        let ctx = RequestContext {
            client_ip: IpAddr::from_str("1.2.3.4").unwrap(),
            sticky_key: Some("session-1".into()),
        };

        let idx1 = balancer.select(&pool, &ctx);
        let idx2 = balancer.select(&pool, &ctx);

        assert_eq!(idx1, idx2);
        assert!(idx1.is_some());
    }

    #[test]
    fn test_sticky_different_keys_different_upstreams() {
        let pool = test_pool();
        let store = Arc::new(CookieSession::new("test".into())) as Arc<dyn SessionStore>;
        let balancer = Sticky::new(
            Arc::new(RoundRobin::new()),
            store.clone(),
            Duration::from_secs(60),
        );

        let ctx1 = RequestContext {
            client_ip: IpAddr::from_str("1.2.3.4").unwrap(),
            sticky_key: Some("session-a".into()),
        };
        let ctx2 = RequestContext {
            client_ip: IpAddr::from_str("1.2.3.4").unwrap(),
            sticky_key: Some("session-b".into()),
        };

        let idx1 = balancer.select(&pool, &ctx1);
        let idx2 = balancer.select(&pool, &ctx2);

        // Two different sessions should get two different upstreams
        // (round-robin fallback)
        assert!(idx1.is_some());
        assert!(idx2.is_some());
        assert_ne!(idx1, idx2);
    }

    #[test]
    fn test_sticky_no_key_falls_through() {
        let pool = test_pool();
        let store = Arc::new(CookieSession::new("test".into())) as Arc<dyn SessionStore>;
        let balancer = Sticky::new(
            Arc::new(RoundRobin::new()),
            store.clone(),
            Duration::from_secs(60),
        );

        let ctx = RequestContext {
            client_ip: IpAddr::from_str("1.2.3.4").unwrap(),
            sticky_key: None,
        };

        let idx = balancer.select(&pool, &ctx);
        assert!(idx.is_some());
        // Without a key, round-robin advances.
        let idx2 = balancer.select(&pool, &ctx);
        assert_ne!(idx, idx2);
    }

    #[test]
    fn test_sticky_reselects_on_unhealthy() {
        let cfg = UpstreamPoolConfig {
            name: "test".into(),
            strategy: crate::config::upstream::Strategy::Sticky,
            upstreams: vec![
                crate::config::upstream::UpstreamConfig {
                    addr: "10.0.0.1:8080".into(),
                    weight: 1,
                    max_connections: None,
                },
                crate::config::upstream::UpstreamConfig {
                    addr: "10.0.0.2:8080".into(),
                    weight: 1,
                    max_connections: None,
                },
            ],
            health: None,
            session: None,
            brain: None,
            connections: None,
        };
        let pool = UpstreamPool::from_config(&cfg);

        // Mark the second upstream unhealthy
        pool.upstreams[1]
            .healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let store = Arc::new(CookieSession::new("test".into())) as Arc<dyn SessionStore>;
        let balancer = Sticky::new(
            Arc::new(RoundRobin::new()),
            store.clone(),
            Duration::from_secs(60),
        );

        // First request pins to upstream[1] (round-robin picks index 0,
        // then index 1)
        let ctx1 = RequestContext {
            client_ip: IpAddr::from_str("1.2.3.4").unwrap(),
            sticky_key: Some("key1".into()),
        };
        let idx1 = balancer.select(&pool, &ctx1).unwrap();
        assert_eq!(idx1, 0); // round-robin picks index 0 first

        // Second request with same key should still get index 0 (healthy)
        let idx2 = balancer.select(&pool, &ctx1).unwrap();
        assert_eq!(idx2, 0);

        // Now mark upstream[0] unhealthy too
        pool.upstreams[0]
            .healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // Third request should reselect — both unhealthy, so None
        let idx3 = balancer.select(&pool, &ctx1);
        assert_eq!(idx3, None);
    }
}
