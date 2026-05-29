pub mod active;
pub mod passive;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tracing::{info, warn};

use crate::balancer::{UpstreamPool, UpstreamState};
use crate::health::active::{run_check, CheckMode, CheckResult};
use crate::observability::metrics;

/// Active health monitor.
///
/// Spawns one background tokio task per upstream that has a health
/// check configured. Each task periodically runs a TCP or HTTP probe
/// and updates the upstream's `healthy` flag atomically.
///
/// The load balancer already filters by `healthy`, so failover is
/// automatic — no further integration required.
pub struct HealthMonitor {
    /// Join handles for all spawned check tasks.
    /// Aborted on drop so new checks can be started on config reload.
    pub handles: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for HealthMonitor {
    fn drop(&mut self) {
        for handle in &self.handles {
            handle.abort();
        }
    }
}

/// Per-upstream check state (private to each task).
struct CheckState {
    consecutive_failures: u32,
    consecutive_successes: u32,
    previously_healthy: bool,
}

impl CheckState {
    fn new() -> Self {
        Self {
            consecutive_failures: 0,
            consecutive_successes: 0,
            previously_healthy: true,
        }
    }

    fn record_result(
        &mut self,
        result: CheckResult,
        state: &UpstreamState,
        unhealthy_threshold: u32,
        healthy_threshold: u32,
    ) {
        match result {
            CheckResult::Pass => {
                self.consecutive_failures = 0;
                self.consecutive_successes += 1;

                if !self.previously_healthy
                    && self.consecutive_successes >= healthy_threshold
                {
                    state.healthy.store(true, std::sync::atomic::Ordering::Relaxed);
                    info!(
                        addr = %state.addr,
                        "upstream recovered — marked healthy"
                    );
                    self.previously_healthy = true;
                }
            }
            CheckResult::Fail(ref reason) => {
                self.consecutive_successes = 0;
                self.consecutive_failures += 1;

                if self.previously_healthy
                    && self.consecutive_failures >= unhealthy_threshold
                {
                    state.healthy.store(false, std::sync::atomic::Ordering::Relaxed);
                    warn!(
                        addr = %state.addr,
                        reason = %reason,
                        consecutive_failures = self.consecutive_failures,
                        "upstream marked unhealthy"
                    );
                    self.previously_healthy = false;
                }
            }
        }
    }
}

impl HealthMonitor {
    /// Start active health checks for all pools that have health
    /// configured.
    ///
    /// Returns a `HealthMonitor` whose dropped handles will cancel
    /// the check tasks on shutdown.
    pub fn start(
        config_pools: &[crate::config::upstream::UpstreamPoolConfig],
        runtime_pools: Arc<Vec<UpstreamPool>>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        let mut handles = Vec::new();

        for cfg_pool in config_pools {
            let health_cfg = match &cfg_pool.health {
                Some(h) => h.clone(),
                None => continue,
            };

            let runtime_pool = match runtime_pools.iter().find(|p| p.name == cfg_pool.name) {
                Some(p) => p,
                None => {
                    warn!(pool = %cfg_pool.name, "health check configured but runtime pool not found");
                    continue;
                }
            };

            let interval = parse_duration_or(&health_cfg.interval, 10);
            let timeout = parse_duration_or(&health_cfg.timeout, 3);
            let mode = if health_cfg.path.is_empty() {
                CheckMode::Tcp
            } else {
                CheckMode::Http {
                    path: health_cfg.path.clone(),
                }
            };

            info!(
                pool = %cfg_pool.name,
                upstream_count = runtime_pool.upstreams.len(),
                interval_s = interval,
                mode = ?mode,
                "health checks started"
            );

            let pool_name_outer = cfg_pool.name.clone();

            for upstream in &runtime_pool.upstreams {
                let state = upstream.clone();
                let addr = state.addr.clone();
                let mode = mode.clone();
                let mut shutdown_rx = shutdown_rx.clone();
                let unhealthy_threshold = health_cfg.unhealthy_threshold;
                let healthy_threshold = health_cfg.healthy_threshold;
                let pool_name = pool_name_outer.clone();

                let handle = tokio::spawn(async move {
                    let mut check = CheckState::new();

                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_secs(interval)) => {
                                let result = run_check(&addr, &mode, Duration::from_secs(timeout)).await;
                                let previously_healthy =
                                    state.healthy.load(Ordering::Relaxed);
                                check.record_result(
                                    result,
                                    &*state,
                                    unhealthy_threshold,
                                    healthy_threshold,
                                );
                                let now_healthy =
                                    state.healthy.load(Ordering::Relaxed);
                                if previously_healthy != now_healthy {
                                    let m = metrics::get();
                                    m.upstream_health
                                        .with_label_values(&[addr.as_str()])
                                        .set(if now_healthy { 1 } else { 0 });
                                    if !now_healthy {
                                        m.failover_total.inc();
                                    }
                                    let pn = pool_name.clone();
                                    crate::observability::realtime::broadcast(std::sync::Arc::new(
                                        crate::observability::realtime::RealtimeEvent::HealthTransition(
                                            crate::observability::realtime::HealthTransition {
                                                addr: addr.clone(),
                                                healthy: now_healthy,
                                                pool_name: pn,
                                                timestamp: chrono::Utc::now().to_rfc3339(),
                                            },
                                        ),
                                    ));
                                }
                            }
                            _ = shutdown_rx.changed() => break,
                        }
                    }
                });

                handles.push(handle);
            }
        }

        Self { handles }
    }
}

/// Parse a duration string or return a fallback value (in seconds).
fn parse_duration_or(s: &str, default: u64) -> u64 {
    crate::config::validate::parse_duration(s).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use crate::balancer::UpstreamState;

    #[test]
    fn check_state_detects_unhealthy() {
        let state = UpstreamState::new("10.0.0.1:3000".into(), 1);
        let mut cs = CheckState::new();

        assert!(state.healthy.load(Ordering::Relaxed));

        // Two failures should NOT trigger (threshold is 3)
        cs.record_result(CheckResult::Fail("err".into()), &state, 3, 2);
        assert!(state.healthy.load(Ordering::Relaxed));
        cs.record_result(CheckResult::Fail("err".into()), &state, 3, 2);
        assert!(state.healthy.load(Ordering::Relaxed));

        // Third failure triggers unhealthy
        cs.record_result(CheckResult::Fail("err".into()), &state, 3, 2);
        assert!(!state.healthy.load(Ordering::Relaxed));
    }

    #[test]
    fn check_state_detects_recovery() {
        let state = UpstreamState::new("10.0.0.1:3000".into(), 1);
        let mut cs = CheckState::new();

        // Mark unhealthy first
        cs.previously_healthy = true;
        state.healthy.store(false, Ordering::Relaxed);
        cs.previously_healthy = false;

        // One success should NOT trigger (threshold is 2)
        cs.record_result(CheckResult::Pass, &state, 3, 2);
        assert!(!state.healthy.load(Ordering::Relaxed));

        // Second success triggers recovery
        cs.record_result(CheckResult::Pass, &state, 3, 2);
        assert!(state.healthy.load(Ordering::Relaxed));
    }

    #[test]
    fn single_failure_resets_on_success() {
        let state = UpstreamState::new("10.0.0.1:3000".into(), 1);
        let mut cs = CheckState::new();

        cs.record_result(CheckResult::Fail("err".into()), &state, 3, 2);
        assert_eq!(cs.consecutive_failures, 1);

        cs.record_result(CheckResult::Pass, &state, 3, 2);
        assert_eq!(cs.consecutive_failures, 0);
        assert_eq!(cs.consecutive_successes, 1);
    }
}
