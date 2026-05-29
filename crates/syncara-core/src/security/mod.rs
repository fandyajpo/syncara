pub mod blocklist;
pub mod config;
pub mod connection_limiter;
pub mod rate_limiter;
pub mod validator;

use std::net::IpAddr;
use std::sync::OnceLock;
use std::time::Duration;

use hyper::{HeaderMap, Request, StatusCode};

pub use self::config::{
    BlocklistConfig, ConnectionLimitsConfig, RateLimitConfig, SecurityConfig,
};
pub use self::connection_limiter::{ConnectionLimiter, ConnPermit, WsPermit};
pub use self::rate_limiter::RateLimiter;
pub use self::blocklist::IpBlocklist;
pub use self::validator::{check_body_size, is_hop_by_hop, strip_hop_by_hop};

/// Global security layer singleton.
static SECURITY: OnceLock<SecurityLayer> = OnceLock::new();

/// Initialise the security layer from config.
pub fn init(cfg: &SecurityConfig) {
    let layer = SecurityLayer::new(cfg);
    let _ = SECURITY.set(layer);
}

/// Access the global security layer.
pub fn get() -> &'static SecurityLayer {
    SECURITY.get().expect("SecurityLayer not initialised — call security::init() first")
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub request: Duration,
    pub upstream: Duration,
    pub websocket: Duration,
    pub header_read: Option<Duration>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            request: Duration::from_secs(30),
            upstream: Duration::from_secs(30),
            websocket: Duration::from_secs(1800),
            header_read: Some(Duration::from_secs(10)),
        }
    }
}

/// Unified security and hardening layer.
pub struct SecurityLayer {
    pub timeouts: TimeoutConfig,
    pub rate_limiter: Option<RateLimiter>,
    pub connection_limiter: ConnectionLimiter,
    pub blocklist: Option<IpBlocklist>,
    pub max_body_size: u64,
    pub keepalive: Option<Duration>,
}

impl SecurityLayer {
    pub fn new(cfg: &SecurityConfig) -> Self {
        let parse_dur = |s: &Option<String>, default: Duration| -> Duration {
            s.as_ref()
                .and_then(|v| crate::config::validate::parse_duration(v).ok())
                .map(Duration::from_secs)
                .unwrap_or(default)
        };

        let timeouts = TimeoutConfig {
            request: parse_dur(&cfg.request_timeout, Duration::from_secs(30)),
            upstream: parse_dur(&cfg.upstream_timeout, Duration::from_secs(30)),
            websocket: parse_dur(&cfg.websocket_timeout, Duration::from_secs(1800)),
            header_read: cfg.http1_header_read_timeout.as_ref().and_then(|s| {
                crate::config::validate::parse_duration(s).ok().map(Duration::from_secs)
            }),
        };

        let rate_limiter = cfg.rate_limit.as_ref().and_then(|rl| {
            if rl.enabled {
                Some(RateLimiter::new(
                    rl.requests_per_minute as usize,
                    Duration::from_secs(60),
                ))
            } else {
                None
            }
        });

        let conn_limits = cfg.connections.clone().unwrap_or(ConnectionLimitsConfig {
            max_active: 10_000,
            websocket_max: 5_000,
            per_upstream: None,
        });

        let connection_limiter =
            ConnectionLimiter::new(conn_limits.max_active, conn_limits.websocket_max);

        let blocklist = cfg.blocklist.as_ref().map(|bl| {
            IpBlocklist::new(
                bl.allowed_cidrs.clone(),
                bl.denied_cidrs.clone(),
                bl.auto_block_after,
                &bl.auto_block_ttl,
            )
        });

        let max_body_size = parse_max_body_size(&cfg.max_body_size);

        let keepalive = cfg.tcp_keepalive.as_ref().and_then(|s| {
            crate::config::validate::parse_duration(s).ok().map(Duration::from_secs)
        });

        Self {
            timeouts,
            rate_limiter,
            connection_limiter,
            blocklist,
            max_body_size,
            keepalive,
        }
    }

    pub fn validate<B>(&self, req: &Request<B>) -> Result<(), StatusCode> {
        validator::validate_request(req)
    }

    /// Check rate limit + blocklist for the given client IP.
    /// If the IP is blocked, returns false.
    /// If rate-limited, records the violation for auto-block.
    pub fn check_rate_limit(&self, client_ip: IpAddr) -> bool {
        // Blocklist check first
        if let Some(bl) = &self.blocklist {
            if !bl.is_allowed(client_ip) {
                return false;
            }
        }

        match &self.rate_limiter {
            Some(rl) => {
                let allowed = rl.check(client_ip);
                if !allowed {
                    if let Some(bl) = &self.blocklist {
                        let violations = rl.violation_count(client_ip);
                        bl.record_violation(client_ip, violations);
                        rl.clear_violations(client_ip);
                    }
                }
                allowed
            }
            None => true,
        }
    }

    /// Check if the request body size is within the configured limit.
    pub fn check_body_size(&self, headers: &HeaderMap) -> bool {
        validator::check_body_size(headers, self.max_body_size)
    }

    pub fn try_acquire_conn(&self) -> Option<ConnPermit> {
        self.connection_limiter.try_acquire_conn()
    }

    pub fn try_acquire_ws(&self) -> Option<WsPermit> {
        self.connection_limiter.try_acquire_ws()
    }
}

fn parse_max_body_size(s: &Option<String>) -> u64 {
    const DEFAULT: u64 = 10 * 1024 * 1024; // 10 MB
    let s = match s.as_ref() {
        Some(v) => v,
        None => return DEFAULT,
    };
    let s = s.trim().to_ascii_lowercase();
    if let Some(val) = s.strip_suffix("mb") {
        if let Ok(n) = val.trim().parse::<u64>() {
            return n * 1024 * 1024;
        }
    }
    if let Some(val) = s.strip_suffix("kb") {
        if let Ok(n) = val.trim().parse::<u64>() {
            return n * 1024;
        }
    }
    if let Some(val) = s.strip_suffix("gb") {
        if let Ok(n) = val.trim().parse::<u64>() {
            return n * 1024 * 1024 * 1024;
        }
    }
    // Plain bytes
    if let Ok(n) = s.parse::<u64>() {
        return n;
    }
    DEFAULT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timeouts_are_reasonable() {
        let t = TimeoutConfig::default();
        assert_eq!(t.request, Duration::from_secs(30));
        assert_eq!(t.upstream, Duration::from_secs(30));
        assert_eq!(t.websocket, Duration::from_secs(1800));
    }

    #[test]
    fn validate_rejects_long_uri() {
        let long = "/".repeat(9000);
        let req = Request::builder()
            .uri(&long)
            .header("host", "x")
            .body(http_body_util::Full::new(bytes::Bytes::new()))
            .unwrap();
        let layer = SecurityLayer::new(&SecurityConfig::default());
        assert_eq!(layer.validate(&req), Err(StatusCode::URI_TOO_LONG));
    }

    #[test]
    fn rate_limiter_disabled_by_default() {
        let layer = SecurityLayer::new(&SecurityConfig::default());
        assert!(layer.rate_limiter.is_none());
        assert!(layer.check_rate_limit("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn connection_limiter_has_default_capacity() {
        let layer = SecurityLayer::new(&SecurityConfig::default());
        assert!(layer.try_acquire_conn().is_some());
        assert!(layer.try_acquire_ws().is_some());
    }

    #[test]
    fn blocklist_blocks_denied_cidr() {
        let mut cfg = SecurityConfig::default();
        cfg.blocklist = Some(BlocklistConfig {
            allowed_cidrs: vec![],
            denied_cidrs: vec!["10.0.0.0/8".into()],
            auto_block_after: None,
            auto_block_ttl: "5m".into(),
        });
        let layer = SecurityLayer::new(&cfg);
        assert!(!layer.check_rate_limit("10.0.0.1".parse().unwrap()));
        assert!(layer.check_rate_limit("192.168.0.1".parse().unwrap()));
    }

    #[test]
    fn body_size_parse() {
        assert_eq!(parse_max_body_size(&Some("10mb".into())), 10 * 1024 * 1024);
        assert_eq!(parse_max_body_size(&Some("500kb".into())), 500 * 1024);
        assert_eq!(parse_max_body_size(&Some("1gb".into())), 1024 * 1024 * 1024);
        assert_eq!(parse_max_body_size(&Some("5242880".into())), 5_242_880);
        assert_eq!(parse_max_body_size(&None), 10 * 1024 * 1024);
    }
}
