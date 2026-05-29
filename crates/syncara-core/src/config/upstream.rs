use crate::config::brain::BrainConfig;
use crate::config::health::HealthCheckConfig;
use crate::config::session::SessionConfig;

#[derive(Debug, Clone, serde::Deserialize)]
pub enum Strategy {
    #[serde(rename = "round-robin")]
    RoundRobin,
    #[serde(rename = "least-connections")]
    LeastConnections,
    #[serde(rename = "weighted")]
    Weighted,
    #[serde(rename = "ip-hash")]
    IpHash,
    #[serde(rename = "sticky")]
    Sticky,
    #[serde(rename = "brain")]
    Brain,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamPoolConfig {
    #[serde(default = "default_pool_name")]
    pub name: String,

    #[serde(default)]
    pub strategy: Strategy,

    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,

    #[serde(default)]
    pub health: Option<HealthCheckConfig>,

    #[serde(default)]
    pub session: Option<SessionConfig>,

    #[serde(default)]
    pub brain: Option<BrainConfig>,

    /// Per-upstream connection limit for this pool (0 = unlimited).
    #[serde(default)]
    pub connections: Option<u32>,
}

fn default_pool_name() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamConfig {
    pub addr: String,

    #[serde(default = "default_weight")]
    pub weight: u32,

    /// Per-upstream connection limit (overrides pool-level `connections`).
    #[serde(default)]
    pub max_connections: Option<u32>,
}

fn default_weight() -> u32 {
    1
}
