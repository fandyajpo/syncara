use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::balancer::{UpstreamPool, UpstreamState};
use crate::config::upstream::Strategy;
use crate::brain::tracker;

static STATUS: std::sync::OnceLock<Arc<tokio::sync::RwLock<StatusSnapshot>>> =
    std::sync::OnceLock::new();

/// Initialize the status snapshot with pool state.
pub fn init(pools: &[UpstreamPool]) {
    let snapshot = StatusSnapshot::from_pools(pools);
    let _ = STATUS.set(Arc::new(tokio::sync::RwLock::new(snapshot)));
}

/// Get the current status snapshot (read-only).
pub fn get() -> &'static Arc<tokio::sync::RwLock<StatusSnapshot>> {
    STATUS.get().expect("status not initialized")
}

/// A snapshot of pool/upstream state.
///
/// Read from `/status` by the admin server and `syncara status`.
pub struct StatusSnapshot {
    pub pools: Vec<PoolStatus>,
    pub version: &'static str,
}

impl StatusSnapshot {
    pub fn from_pools(pools: &[UpstreamPool]) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            pools: pools.iter().map(PoolStatus::from_pool).collect(),
        }
    }
}

pub struct PoolStatus {
    pub name: String,
    pub strategy: String,
    pub upstreams: Vec<UpstreamStatus>,
}

impl PoolStatus {
    fn from_pool(pool: &UpstreamPool) -> Self {
        Self {
            name: pool.name.clone(),
            strategy: strategy_name(&pool.strategy),
            upstreams: pool.upstreams.iter().map(UpstreamStatus::from_state).collect(),
        }
    }
}

pub struct UpstreamStatus {
    pub addr: String,
    pub weight: u32,
    pub healthy: bool,
    pub active_connections: u64,
    pub latency_ms: f64,
}

impl UpstreamStatus {
    fn from_state(state: &Arc<UpstreamState>) -> Self {
        let latency = tracker::get()
            .p50(&state.addr)
            .map(|s| s * 1000.0)
            .unwrap_or(0.0);

        Self {
            addr: state.addr.clone(),
            weight: state.weight,
            healthy: state.healthy.load(Ordering::Relaxed),
            active_connections: state.active_connections.load(Ordering::Relaxed),
            latency_ms: latency,
        }
    }
}

fn strategy_name(s: &Strategy) -> String {
    match s {
        Strategy::RoundRobin => "round-robin".into(),
        Strategy::LeastConnections => "least-connections".into(),
        Strategy::Weighted => "weighted".into(),
        Strategy::IpHash => "ip-hash".into(),
        Strategy::Sticky => "sticky".into(),
        Strategy::Brain => "brain".into(),
    }
}
