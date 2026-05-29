use std::sync::OnceLock;

use prometheus::{
    HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, opts, Registry,
};

/// Global metrics singleton, initialised once at startup.
static METRICS: OnceLock<AppMetrics> = OnceLock::new();

pub fn init() -> &'static AppMetrics {
    METRICS.get_or_init(AppMetrics::new)
}

pub fn get() -> &'static AppMetrics {
    METRICS.get().expect("AppMetrics not initialised — call observability::metrics::init() first")
}

/// All application-level metric handles.
pub struct AppMetrics {
    pub registry: Registry,

    // ── Traffic ────────────────────────────────────────────
    /// Total HTTP requests received.
    pub requests_total: IntCounter,

    /// Total responses sent, labelled by status class ("2xx", "4xx", "5xx", "error").
    pub responses_total: IntCounterVec,

    /// Currently in-flight HTTP requests.
    pub requests_active: IntGauge,

    /// Request latency in seconds.
    pub latency_seconds: HistogramVec,

    // ── WebSocket ──────────────────────────────────────────
    /// Total WebSocket upgrade requests handled.
    pub ws_upgrades_total: IntCounter,

    /// Currently active WebSocket tunnnels.
    pub ws_connections_active: IntGauge,

    // ── Health ─────────────────────────────────────────────
    /// Per-upstream health: 1 = healthy, 0 = unhealthy.
    pub upstream_health: IntGaugeVec,

    /// Total failover events (upstream marked unhealthy).
    pub failover_total: IntCounter,

    // ── Runtime ────────────────────────────────────────────
    /// Config reload attempts and their outcomes.
    pub config_reloads_total: IntCounterVec,
}

impl AppMetrics {
    fn new() -> Self {
        let registry = Registry::new();

        let requests_total = IntCounter::new(
            "syncara_requests_total",
            "Total HTTP requests received",
        )
        .expect("metric def");

        let responses_total = IntCounterVec::new(
            opts!("syncara_responses_total", "Total responses by status class"),
            &["status_class"],
        )
        .expect("metric def");

        let requests_active = IntGauge::new(
            "syncara_requests_active",
            "Currently in-flight requests",
        )
        .expect("metric def");

        let latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "syncara_latency_seconds",
                "Request latency in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["listener", "upstream"],
        )
        .expect("metric def");

        let ws_upgrades_total = IntCounter::new(
            "syncara_ws_upgrades_total",
            "Total WebSocket upgrade requests",
        )
        .expect("metric def");

        let ws_connections_active = IntGauge::new(
            "syncara_ws_connections_active",
            "Currently active WebSocket tunnels",
        )
        .expect("metric def");

        let upstream_health = IntGaugeVec::new(
            opts!("syncara_upstream_health", "Upstream health (1=healthy, 0=unhealthy)"),
            &["upstream"],
        )
        .expect("metric def");

        let failover_total = IntCounter::new(
            "syncara_failover_total",
            "Total upstream failover events",
        )
        .expect("metric def");

        let config_reloads_total = IntCounterVec::new(
            opts!("syncara_config_reloads_total", "Config reload attempts by result"),
            &["result"],
        )
        .expect("metric def");

        // Register all metrics.
        registry.register(Box::new(requests_total.clone())).ok();
        registry.register(Box::new(responses_total.clone())).ok();
        registry.register(Box::new(requests_active.clone())).ok();
        registry.register(Box::new(latency_seconds.clone())).ok();
        registry.register(Box::new(ws_upgrades_total.clone())).ok();
        registry.register(Box::new(ws_connections_active.clone())).ok();
        registry.register(Box::new(upstream_health.clone())).ok();
        registry.register(Box::new(failover_total.clone())).ok();
        registry.register(Box::new(config_reloads_total.clone())).ok();

        Self {
            registry,
            requests_total,
            responses_total,
            requests_active,
            latency_seconds,
            ws_upgrades_total,
            ws_connections_active,
            upstream_health,
            failover_total,
            config_reloads_total,
        }
    }
}
