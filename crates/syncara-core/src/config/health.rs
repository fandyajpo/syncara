/// Health check configuration for an upstream pool.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthCheckConfig {
    /// HTTP path to probe (default: "/health").
    #[serde(default = "default_health_path")]
    pub path: String,

    /// Interval between checks (default: "10s").
    #[serde(default = "default_health_interval")]
    pub interval: String,

    /// Per-check timeout (default: "3s").
    #[serde(default = "default_health_timeout")]
    pub timeout: String,

    /// Consecutive failures to mark unhealthy (default: 2).
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,

    /// Consecutive successes to mark healthy (default: 1).
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,
}

fn default_health_path() -> String {
    "/health".to_string()
}

fn default_health_interval() -> String {
    "10s".to_string()
}

fn default_health_timeout() -> String {
    "3s".to_string()
}

fn default_unhealthy_threshold() -> u32 {
    2
}

fn default_healthy_threshold() -> u32 {
    1
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            path: default_health_path(),
            interval: default_health_interval(),
            timeout: default_health_timeout(),
            unhealthy_threshold: default_unhealthy_threshold(),
            healthy_threshold: default_healthy_threshold(),
        }
    }
}

/// Passive health tracking configuration.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveCheckConfig {
    /// Consecutive failures before ejection (default: 5).
    #[serde(default = "default_passive_max_fails")]
    pub max_fails: u32,

    /// Time before retrying an ejected upstream (default: "30s").
    #[serde(default = "default_passive_fail_timeout")]
    pub fail_timeout: String,
}

fn default_passive_max_fails() -> u32 {
    5
}

fn default_passive_fail_timeout() -> String {
    "30s".to_string()
}

impl Default for PassiveCheckConfig {
    fn default() -> Self {
        Self {
            max_fails: default_passive_max_fails(),
            fail_timeout: default_passive_fail_timeout(),
        }
    }
}
