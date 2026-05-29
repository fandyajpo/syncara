/// Brain strategy configuration.
///
/// Controls which signals the adaptive routing engine considers when
/// scoring and selecting upstream targets.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrainConfig {
    /// Prefer upstreams with lower response latency.
    #[serde(default = "default_true")]
    pub latency_aware: bool,

    /// Exclude / penalise degraded upstreams.
    #[serde(default = "default_true")]
    pub health_aware: bool,

    /// Avoid upstreams with high WebSocket connection pressure.
    #[serde(default = "default_false")]
    pub websocket_pressure_aware: bool,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            latency_aware: true,
            health_aware: true,
            websocket_pressure_aware: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}
