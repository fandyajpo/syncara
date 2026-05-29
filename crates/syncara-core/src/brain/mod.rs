pub mod tracker;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::info;

use crate::balancer::{LoadBalancer, RequestContext, UpstreamPool};
use crate::config::brain::BrainConfig;

// ── Scoring model constants ─────────────────────────────────────

/// Base score every upstream starts with.
const BASE_SCORE: i64 = 100;

/// Deduction when an upstream is marked unhealthy (binary health).
const UNHEALTHY_DEDUCTION: i64 = 100;

/// Deduction when an upstream is degraded (latency > 2× global p50).
const DEGRADED_DEDUCTION: i64 = 30;

/// Load pressure: factor applied when active_conns / weight > 0.8.
const LOAD_PRESSURE_FACTOR: i64 = 20;

/// Additional deduction when latency > 2× global p50.
const LATENCY_SLOW_DEDUCTION: i64 = 25;

/// Additional deduction when latency > 3× global p50.
const LATENCY_VERY_SLOW_DEDUCTION: i64 = 10;

/// High load cap.
const LOAD_CAP: i64 = 40;

/// Load ratio threshold (80%).
const LOAD_RATIO_THRESHOLD: f64 = 0.8;

// ── Data types ──────────────────────────────────────────────────

/// A single deduction applied to a target's score.
#[derive(Debug, Clone)]
pub struct Deduction {
    pub reason: String,
    pub points: i64,
}

/// Score breakdown for a single upstream target.
#[derive(Debug, Clone)]
pub struct TargetScore {
    pub addr: String,
    pub score: i64,
    pub deductions: Vec<Deduction>,
}

/// A fully explainable routing decision.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub selected: String,
    pub explanation: String,
    pub scores: Vec<TargetScore>,
    pub pool_name: String,
}

// ── Brain balancer ──────────────────────────────────────────────

/// Adaptive load balancer that scores upstreams based on runtime
/// signals (health, load, latency, WebSocket pressure) and selects
/// the best candidate.
///
/// Every decision is deterministic and explainable — no machine
/// learning, no black boxes.
pub struct BrainBalancer {
    config: BrainConfig,
    /// Tiebreaker counter: when multiple upstreams share the same
    /// score we round-robin among the leaders.
    tiebreaker: AtomicU64,
}

impl BrainBalancer {
    pub fn new(config: BrainConfig) -> Self {
        Self {
            config,
            tiebreaker: AtomicU64::new(0),
        }
    }

    /// Score every upstream in the pool.
    fn score_targets(&self, pool: &UpstreamPool) -> Vec<TargetScore> {
        let tracker = tracker::get();
        let global_p50 = tracker.global_p50();

        let mut scores: Vec<TargetScore> = pool
            .upstreams
            .iter()
            .map(|state| {
                let addr = &state.addr;
                let mut score = BASE_SCORE;
                let mut deductions: Vec<Deduction> = Vec::new();

                // ── 1. Health signal ──────────────────────────
                if self.config.health_aware {
                    let healthy = state.healthy.load(Ordering::Relaxed);
                    if !healthy {
                        deductions.push(Deduction {
                            reason: format!("{addr} marked unhealthy"),
                            points: UNHEALTHY_DEDUCTION,
                        });
                        score -= UNHEALTHY_DEDUCTION;
                    }
                }

                // ── 2. Load pressure (always active) ──────────
                let active_conns = state.active_connections.load(Ordering::Relaxed);
                let weight = state.weight.max(1);
                let load_ratio = active_conns as f64 / weight as f64;
                if load_ratio > LOAD_RATIO_THRESHOLD {
                    let load_deduction = (LOAD_PRESSURE_FACTOR as f64
                        * (load_ratio / LOAD_RATIO_THRESHOLD))
                        .min(LOAD_CAP as f64) as i64;
                    deductions.push(Deduction {
                        reason: format!(
                            "{addr} high load: {active_conns} active / weight {weight} = {load_ratio:.2}"
                        ),
                        points: load_deduction,
                    });
                    score -= load_deduction;
                }

                // ── 3. Latency signal ─────────────────────────
                if self.config.latency_aware {
                    if let (Some(gp50), Some(up_p50)) = (global_p50, tracker.p50(addr)) {
                        if up_p50 > gp50 * 2.0 {
                            deductions.push(Deduction {
                                reason: format!(
                                    "{addr} degraded: p50={up_p50:.1}ms > 2× global p50={gp50:.1}ms"
                                ),
                                points: DEGRADED_DEDUCTION,
                            });
                            score -= DEGRADED_DEDUCTION;

                            if up_p50 > gp50 * 3.0 {
                                deductions.push(Deduction {
                                    reason: format!(
                                        "{addr} very slow: p50={up_p50:.1}ms > 3× global p50={gp50:.1}ms"
                                    ),
                                    points: LATENCY_VERY_SLOW_DEDUCTION,
                                });
                                score -= LATENCY_VERY_SLOW_DEDUCTION;
                            }
                        } else if up_p50 > gp50 * 1.5 {
                            deductions.push(Deduction {
                                reason: format!(
                                    "{addr} slow: p50={up_p50:.1}ms > 1.5× global p50={gp50:.1}ms"
                                ),
                                points: LATENCY_SLOW_DEDUCTION,
                            });
                            score -= LATENCY_SLOW_DEDUCTION;
                        }
                    }
                }

                // ── 4. WebSocket pressure ─────────────────────
                if self.config.websocket_pressure_aware {
                    // WebSocket connections share the same
                    // active_connections counter.  The load pressure
                    // deduction above already captures this.  A
                    // separate WS-specific threshold could be added
                    // here if a dedicated per-upstream WS counter is
                    // plumbed in the future.
                }

                // Clamp so unhealthy upstreams don't go massively
                // negative (aids readability of explanations).
                let score = score.max(0);

                TargetScore {
                    addr: addr.clone(),
                    score,
                    deductions,
                }
            })
            .collect();

        // Sort by score descending for stable explanations.
        scores.sort_by(|a, b| b.score.cmp(&a.score));
        scores
    }

    /// Select the best target and build an explanation.
    pub fn decide(&self, pool: &UpstreamPool) -> Option<RoutingDecision> {
        let pool_name = pool.name.clone();
        let scores = self.score_targets(pool);

        // Find the highest score among healthy (non-zero) targets.
        let best_score = scores.iter().map(|s| s.score).max()?;
        if best_score == 0 {
            return None;
        }

        // All targets tied at the top score.
        let leaders: Vec<&TargetScore> = scores
            .iter()
            .filter(|s| s.score == best_score)
            .collect();

        // Tiebreak: round-robin among leaders.
        let count = self.tiebreaker.fetch_add(1, Ordering::Relaxed);
        let selected = leaders[(count as usize) % leaders.len()];

        let deduction_summary: Vec<String> = selected
            .deductions
            .iter()
            .map(|d| format!("-{} ({})", d.points, d.reason))
            .collect();

        let explanation = if deduction_summary.is_empty() {
            format!(
                "selected {} with score {} — no deductions",
                selected.addr, selected.score
            )
        } else {
            format!(
                "selected {} with score {}: {}",
                selected.addr,
                selected.score,
                deduction_summary.join("; ")
            )
        };

        Some(RoutingDecision {
            selected: selected.addr.clone(),
            explanation,
            scores,
            pool_name,
        })
    }
}

impl LoadBalancer for BrainBalancer {
    fn select(&self, pool: &UpstreamPool, _ctx: &RequestContext) -> Option<usize> {
        let decision = self.decide(pool)?;

        info!(
            pool = %pool.name,
            selected = %decision.selected,
            scores = ?decision.scores.iter().map(|s| format!("{}:{}", s.addr, s.score)).collect::<Vec<_>>(),
            "brain routing decision"
        );

        // Broadcast to realtime dashboard subscribers.
        let arc = Arc::new(crate::observability::realtime::RealtimeEvent::RoutingDecision(
            Arc::new(decision.clone()),
        ));
        crate::observability::realtime::broadcast(arc);

        // Find the index of the selected upstream in the pool.
        pool.upstreams
            .iter()
            .position(|u| u.addr == decision.selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_pool(addrs: &[&str], strategy: crate::config::upstream::Strategy) -> crate::config::upstream::UpstreamPoolConfig {
        crate::config::upstream::UpstreamPoolConfig {
            name: "brain-test".into(),
            strategy,
            upstreams: addrs
                .iter()
                .map(|a| crate::config::upstream::UpstreamConfig {
                    addr: a.to_string(),
                    weight: 1,
                    max_connections: None,
                })
                .collect(),
            health: None,
            session: None,
            connections: None,
            brain: Some(BrainConfig {
                latency_aware: false,
                health_aware: true,
                websocket_pressure_aware: false,
            }),
        }
    }

    fn make_pool(addrs: &[&str]) -> UpstreamPool {
        crate::brain::tracker::init();
        UpstreamPool::from_config(&cfg_pool(addrs, crate::config::upstream::Strategy::Brain))
    }

    #[test]
    fn selects_healthy_over_unhealthy() {
        let pool = make_pool(&["10.0.0.1:3000", "10.0.0.2:3000"]);
        pool.upstreams[1]
            .healthy
            .store(false, Ordering::Relaxed);

        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());
        let idx = pool.acquire(&ctx).unwrap();
        assert_eq!(idx.0, "10.0.0.1:3000");
    }

    #[test]
    fn returns_none_when_all_unhealthy() {
        let pool = make_pool(&["10.0.0.1:3000", "10.0.0.2:3000"]);
        pool.upstreams[0]
            .healthy
            .store(false, Ordering::Relaxed);
        pool.upstreams[1]
            .healthy
            .store(false, Ordering::Relaxed);

        let ctx = RequestContext::new("127.0.0.1".parse().unwrap());
        assert!(pool.acquire(&ctx).is_none());
    }

    #[test]
    fn prefers_lower_load() {
        let pool = make_pool(&["10.0.0.1:3000", "10.0.0.2:3000"]);
        // Simulate high load on upstream 0.
        pool.upstreams[0]
            .active_connections
            .store(50, Ordering::Relaxed);
        pool.upstreams[1]
            .active_connections
            .store(1, Ordering::Relaxed);

        let brain = BrainBalancer::new(BrainConfig {
            latency_aware: false,
            health_aware: true,
            websocket_pressure_aware: false,
        });

        let decision = brain.decide(&pool).unwrap();
        assert_eq!(decision.selected, "10.0.0.2:3000");
    }

    #[test]
    fn decision_is_explainable() {
        let pool = make_pool(&["10.0.0.1:3000"]);
        pool.upstreams[0]
            .active_connections
            .store(100, Ordering::Relaxed);

        let brain = BrainBalancer::new(BrainConfig {
            latency_aware: false,
            health_aware: true,
            websocket_pressure_aware: false,
        });

        let decision = brain.decide(&pool).unwrap();
        assert_eq!(decision.selected, "10.0.0.1:3000");
        assert!(!decision.explanation.is_empty());
        assert!(!decision.scores.is_empty());
        // With high load, there should be a deduction.
        assert!(!decision.scores[0].deductions.is_empty());
    }

    #[test]
    fn all_equal_scores_tiebreak_round_robin() {
        let pool = make_pool(&["10.0.0.1:3000", "10.0.0.2:3000"]);

        let brain = BrainBalancer::new(BrainConfig {
            latency_aware: false,
            health_aware: false,
            websocket_pressure_aware: false,
        });

        let d1 = brain.decide(&pool).unwrap();
        let d2 = brain.decide(&pool).unwrap();
        // With two equal-score targets, round-robin alternates.
        assert_ne!(d1.selected, d2.selected);
    }

    #[test]
    fn latencies_are_not_required() {
        // When no latencies have been recorded, the brain should
        // still function (latency_aware just has no data to act on).
        let pool = make_pool(&["10.0.0.1:3000", "10.0.0.2:3000"]);

        let brain = BrainBalancer::new(BrainConfig {
            latency_aware: true,
            health_aware: false,
            websocket_pressure_aware: false,
        });

        let decision = brain.decide(&pool);
        assert!(decision.is_some());
    }
}
