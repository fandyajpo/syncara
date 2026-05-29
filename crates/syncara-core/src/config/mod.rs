pub mod admin;
pub mod brain;
pub mod health;
pub mod listener;
pub mod session;
pub mod upstream;
pub mod validate;

use std::path::Path;

pub use crate::security::SecurityConfig;
pub use self::admin::{AdminConfig, ManagementAuth};
pub use self::brain::BrainConfig;
pub use self::health::{HealthCheckConfig, PassiveCheckConfig};
pub use self::listener::ListenerConfig;
pub use self::session::SessionConfig;
pub use self::upstream::{Strategy, UpstreamConfig, UpstreamPoolConfig};

/// Top-level Syncara configuration.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub listeners: Vec<ListenerConfig>,

    #[serde(default)]
    pub routes: Vec<RouteConfig>,

    #[serde(default)]
    pub pools: Vec<UpstreamPoolConfig>,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub admin: AdminConfig,

    #[serde(default)]
    pub security: SecurityConfig,
}

/// A single route rule mapping requests to a pool.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteConfig {
    #[serde(default)]
    pub host: Option<String>,

    #[serde(default)]
    pub path: Option<String>,

    /// Pool name reference — always populated after normalization.
    #[serde(default)]
    pub pool: String,

    /// Shorthand: direct upstream URL (e.g. "http://localhost:3000").
    /// Consumed by `normalize()` and converted into a synthetic pool entry.
    #[serde(default)]
    pub proxy: Option<String>,

    #[serde(default)]
    pub websocket: bool,
}

/// Logging configuration.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listeners: vec![],
            routes: vec![],
            pools: vec![],
            logging: LoggingConfig::default(),
            admin: AdminConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// Build a minimal config from CLI flags (zero-config mode).
///
/// Creates one listener on `port` proxying to `backend`.
/// Used by `syncara start --backend localhost:9001`.
pub fn quick_config(backend: &str, port: u16) -> Config {
    let addr = parse_proxy_url(backend);
    Config {
        listeners: vec![crate::config::listener::ListenerConfig {
            host: "0.0.0.0".into(),
            port,
            tls: None,
        }],
        routes: vec![RouteConfig {
            host: None,
            path: Some("/".into()),
            pool: "_default".into(),
            proxy: None,
            websocket: false,
        }],
        pools: vec![UpstreamPoolConfig {
            name: "_default".into(),
            strategy: Strategy::RoundRobin,
            upstreams: vec![UpstreamConfig { addr, weight: 1, max_connections: None }],
            health: None,
            session: None,
            brain: None,
            connections: None,
        }],
        logging: LoggingConfig::default(),
        admin: AdminConfig::default(),
        security: SecurityConfig::default(),
    }
}

/// Load configuration from a YAML file.
///
/// Parses the raw YAML, then normalizes shorthand fields (e.g. `proxy`)
/// into full pool entries before returning.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read config file '{}': {}", path.display(), e))?;

    let mut cfg: Config = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse config file '{}': {}", path.display(), e))?;

    normalize(&mut cfg);
    Ok(cfg)
}

/// Run semantic validation rules against the loaded config.
pub fn validate(cfg: &Config) -> anyhow::Result<()> {
    validate::validate_config(cfg)
}

/// Normalize configuration: expand shorthand fields into full entries.
///
/// For each route with `proxy` set, generate a pool name and auto-create
/// a pool entry with that single upstream. After normalization every route
/// has its `pool` field populated and the corresponding pool exists.
fn normalize(cfg: &mut Config) {
    let mut synthetic_pools: Vec<UpstreamPoolConfig> = Vec::new();

    for (i, route) in cfg.routes.iter_mut().enumerate() {
        if let Some(proxy_url) = route.proxy.take() {
            let addr = parse_proxy_url(&proxy_url);
            let pool_name = format!("_route_{}", i);
            route.pool = pool_name.clone();

            synthetic_pools.push(UpstreamPoolConfig {
                name: pool_name,
                strategy: Strategy::RoundRobin,
                upstreams: vec![UpstreamConfig { addr, weight: 1, max_connections: None }],
                health: None,
                session: None,
                brain: None,
                connections: None,
            });
        }
    }

    cfg.pools.extend(synthetic_pools);
}

/// Generate a default config YAML for the `init` command.
pub fn default_config_yaml() -> String {
    r#"# Syncara Configuration
# https://syncara.ai/docs

listeners:
  - port: 8080

routes:
  - path: /
    proxy: http://localhost:3000

logging:
  level: info
  format: text
"#
    .to_string()
}

/// Extract "host:port" from a proxy URL like "http://localhost:3000".
pub(crate) fn parse_proxy_url(url: &str) -> String {
    let url = url.trim();
    // Strip "http://" or "https://" prefix
    let after_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    // Strip trailing path/query — take only host:port
    after_scheme
        .split('/')
        .next()
        .unwrap_or(after_scheme)
        .to_string()
}
