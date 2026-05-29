pub mod ip_hash;
pub mod least_connections;
pub mod round_robin;
pub mod sticky;
pub mod weighted;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::brain::BrainBalancer;
use crate::config::session::SessionConfig;
use crate::config::upstream::{Strategy, UpstreamPoolConfig};

pub type UpstreamAddr = String;

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub client_ip: std::net::IpAddr,
    pub sticky_key: Option<String>,
}

impl RequestContext {
    pub fn new(client_ip: std::net::IpAddr) -> Self {
        Self {
            client_ip,
            sticky_key: None,
        }
    }
}

/// Runtime state for a single upstream.
pub struct UpstreamState {
    pub addr: UpstreamAddr,
    pub weight: u32,
    pub active_connections: AtomicU64,
    pub healthy: AtomicBool,
    /// Max concurrent connections (0 = unlimited).
    pub max_connections: u32,
}

impl UpstreamState {
    pub fn new(addr: UpstreamAddr, weight: u32) -> Self {
        Self {
            addr,
            weight,
            active_connections: AtomicU64::new(0),
            healthy: AtomicBool::new(true),
            max_connections: 0,
        }
    }

    pub fn new_with_limit(addr: UpstreamAddr, weight: u32, max_connections: u32) -> Self {
        Self {
            addr,
            weight,
            active_connections: AtomicU64::new(0),
            healthy: AtomicBool::new(true),
            max_connections,
        }
    }

    /// Returns `true` if this upstream has reached its connection limit.
    pub fn at_capacity(&self) -> bool {
        if self.max_connections == 0 {
            return false;
        }
        self.active_connections.load(Ordering::Relaxed) >= self.max_connections as u64
    }
}

/// RAII guard that decrements `active_connections` on drop.
pub struct BusyGuard {
    state: Arc<UpstreamState>,
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.state.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

/// The common interface for all load balancing strategies.
pub trait LoadBalancer: Send + Sync {
    fn select(&self, pool: &UpstreamPool, ctx: &RequestContext) -> Option<usize>;
}

/// A pool of upstreams with an associated balancing strategy.
#[derive(Clone)]
pub struct UpstreamPool {
    pub name: String,
    pub strategy: Strategy,
    pub upstreams: Vec<Arc<UpstreamState>>,
    pub session_config: Option<SessionConfig>,
    balancer: Arc<dyn LoadBalancer>,
}

impl UpstreamPool {
    pub fn from_config(cfg: &UpstreamPoolConfig) -> Self {
        use std::sync::Arc as StdArc;
        use std::time::Duration;

        let upstreams: Vec<Arc<UpstreamState>> = cfg
            .upstreams
            .iter()
            .map(|u| {
                let max_conn = u.max_connections.unwrap_or(
                    cfg.connections.unwrap_or(0),
                );
                Arc::new(UpstreamState::new_with_limit(
                    u.addr.clone(),
                    u.weight,
                    max_conn,
                ))
            })
            .collect();

        let session_config = cfg.session.clone();

        let balancer: Arc<dyn LoadBalancer> = match cfg.strategy {
            Strategy::RoundRobin => Arc::new(round_robin::RoundRobin::new()),
            Strategy::LeastConnections => Arc::new(least_connections::LeastConnections),
            Strategy::Weighted => Arc::new(weighted::WeightedRoundRobin::new(&upstreams)),
            Strategy::IpHash => Arc::new(ip_hash::IpHash),
            Strategy::Brain => {
                let brain_cfg = cfg.brain.clone().unwrap_or_default();
                Arc::new(BrainBalancer::new(brain_cfg))
            }
            Strategy::Sticky => {
                let fallback: Arc<dyn LoadBalancer> =
                    Arc::new(round_robin::RoundRobin::new());

                let ttl = session_config
                    .as_ref()
                    .and_then(|sc| {
                        let secs = crate::config::validate::parse_duration(&sc.ttl).ok()?;
                        Some(Duration::from_secs(secs))
                    })
                    .unwrap_or(Duration::from_secs(86400));

                let store: StdArc<dyn crate::session::SessionStore> = match session_config
                    .as_ref()
                    .map(|sc| &sc.sticky_type)
                {
                    Some(crate::config::session::StickyType::IpHash) => {
                        StdArc::new(crate::session::ip_hash::IpHashSession::new())
                    }
                    _ => StdArc::new(crate::session::cookie::CookieSession::new(
                        session_config
                            .as_ref()
                            .map(|sc| sc.cookie_name.clone())
                            .unwrap_or_else(|| "_syncara_session".into()),
                    )),
                };

                Arc::new(sticky::Sticky::new(fallback, store, ttl))
            }
        };

        Self {
            name: cfg.name.clone(),
            strategy: cfg.strategy.clone(),
            upstreams,
            session_config,
            balancer,
        }
    }

    /// Acquire an upstream for the given request context.
    ///
    /// Respects per-upstream `max_connections`: if the selected upstream
    /// is at capacity, it scans for any other healthy, non-full upstream.
    /// Returns `None` only when every healthy upstream is at capacity.
    pub fn acquire(&self, ctx: &RequestContext) -> Option<(UpstreamAddr, BusyGuard)> {
        // Try the balancer's selection first.
        if let Some(idx) = self.balancer.select(self, ctx) {
            let state = &self.upstreams[idx];
            if !state.at_capacity() {
                state.active_connections.fetch_add(1, Ordering::Relaxed);
                return Some((
                    state.addr.clone(),
                    BusyGuard {
                        state: state.clone(),
                    },
                ));
            }
        }

        // Fallback: scan for any healthy upstream that isn't at capacity.
        for state in &self.upstreams {
            if state.healthy.load(Ordering::Relaxed) && !state.at_capacity() {
                state.active_connections.fetch_add(1, Ordering::Relaxed);
                return Some((
                    state.addr.clone(),
                    BusyGuard {
                        state: state.clone(),
                    },
                ));
            }
        }

        None
    }

    pub fn healthy_count(&self) -> usize {
        self.upstreams
            .iter()
            .filter(|u| u.healthy.load(Ordering::Relaxed))
            .count()
    }
}
