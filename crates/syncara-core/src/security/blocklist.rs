use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::validate::parse_duration;

/// IP blocklist with CIDR allow/deny and auto-block on rate-limit
/// violation.
///
/// ## Allowed CIDRs
/// If non-empty, *only* IPs matching an allowed CIDR are permitted.
/// All others are denied.
///
/// ## Denied CIDRs
/// IPs matching a denied CIDR are always rejected — checked before
/// the allow list so a deny overrides an allow.
///
/// ## Auto-block
/// After `auto_block_after` rate-limit violations within the rate-limit
/// window, the IP is automatically blocked for `auto_block_ttl`.
pub struct IpBlocklist {
    allowed: Vec<Cidr>,
    denied: Vec<Cidr>,
    auto_block_after: Option<u32>,
    auto_block_ttl: Duration,

    /// Auto-blocked IPs mapped to their unblock time.
    blocked: Mutex<HashMap<IpAddr, Instant>>,
}

impl IpBlocklist {
    pub fn new(
        allowed_cidrs: Vec<String>,
        denied_cidrs: Vec<String>,
        auto_block_after: Option<u32>,
        auto_block_ttl: &str,
    ) -> Self {
        Self {
            allowed: allowed_cidrs.iter().filter_map(|c| Cidr::parse(c).ok()).collect(),
            denied: denied_cidrs.iter().filter_map(|c| Cidr::parse(c).ok()).collect(),
            auto_block_after,
            auto_block_ttl: Duration::from_secs(parse_duration(auto_block_ttl).unwrap_or(300)),
            blocked: Mutex::new(HashMap::new()),
        }
    }

    /// Check whether `ip` is permitted.  Returns `true` if the request
    /// should be allowed, `false` if it should be rejected.
    pub fn is_allowed(&self, ip: IpAddr) -> bool {
        let now = Instant::now();

        // ── Evict expired auto-blocks ─────────────────────────
        if let Ok(mut b) = self.blocked.lock() {
            b.retain(|_, expires| *expires > now);
            if b.contains_key(&ip) {
                return false;
            }
        }

        // ── Denied CIDRs (checked first — explicit deny wins) ─
        for cidr in &self.denied {
            if cidr.matches(ip) {
                return false;
            }
        }

        // ── Allowed CIDRs (if any — restrictive mode) ─────────
        if !self.allowed.is_empty() {
            return self.allowed.iter().any(|c| c.matches(ip));
        }

        true
    }

    /// Called when a rate-limit violation is detected for `ip`.
    /// If `auto_block_after` is set and violations exceed the threshold,
    /// the IP is auto-blocked.
    pub fn record_violation(&self, ip: IpAddr, violation_count: u32) {
        let threshold = match self.auto_block_after {
            Some(n) => n,
            None => return,
        };
        if violation_count >= threshold {
            if let Ok(mut b) = self.blocked.lock() {
                let expires = Instant::now() + self.auto_block_ttl;
                b.insert(ip, expires);
                tracing::warn!(%ip, ttl_s = %self.auto_block_ttl.as_secs(), "ip auto-blocked after rate-limit violations");
            }
        }
    }
}

/// Simple CIDR matcher for IPv4 and IPv6.
struct Cidr {
    addr: u128,
    mask: u128,
    is_v4: bool,
}

impl Cidr {
    fn parse(s: &str) -> Result<Self, ()> {
        let s = s.trim();
        let (ip_str, prefix_len) = match s.split_once('/') {
            Some((ip, pfx)) => (ip, pfx.parse::<u32>().map_err(|_| ())?),
            None => (s, 32),
        };

        let addr: IpAddr = ip_str.parse().map_err(|_| ())?;
        match addr {
            IpAddr::V4(v4) => {
                let num = u128::from(u32::from(v4));
                let mask = if prefix_len >= 32 { !0u128 } else { (!0u128) << (32 - prefix_len) };
                Ok(Self { addr: num, mask, is_v4: true })
            }
            IpAddr::V6(v6) => {
                let num = u128::from(v6);
                let mask = if prefix_len >= 128 { !0u128 } else { (!0u128) << (128 - prefix_len) };
                Ok(Self { addr: num, mask, is_v4: false })
            }
        }
    }

    fn matches(&self, ip: IpAddr) -> bool {
        let num = match ip {
            IpAddr::V4(v4) => {
                if !self.is_v4 { return false; }
                u128::from(u32::from(v4))
            }
            IpAddr::V6(v6) => {
                if self.is_v4 { return false; }
                u128::from(v6)
            }
        };
        (num & self.mask) == (self.addr & self.mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidr_v4_exact_match() {
        let c = Cidr::parse("10.0.0.1/32").unwrap();
        assert!(c.matches("10.0.0.1".parse().unwrap()));
        assert!(!c.matches("10.0.0.2".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_subnet_match() {
        let c = Cidr::parse("10.0.0.0/24").unwrap();
        assert!(c.matches("10.0.0.1".parse().unwrap()));
        assert!(c.matches("10.0.0.255".parse().unwrap()));
        assert!(!c.matches("10.0.1.1".parse().unwrap()));
    }

    #[test]
    fn cidr_v6_match() {
        let c = Cidr::parse("::1/128").unwrap();
        assert!(c.matches("::1".parse().unwrap()));
        assert!(!c.matches("::2".parse().unwrap()));
    }

    #[test]
    fn allow_all_if_no_cidrs() {
        let bl = IpBlocklist::new(vec![], vec![], None, "5m");
        assert!(bl.is_allowed("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn deny_cidr_rejected() {
        let bl = IpBlocklist::new(vec![], vec!["10.0.0.0/8".into()], None, "5m");
        assert!(!bl.is_allowed("10.0.0.1".parse().unwrap()));
        assert!(bl.is_allowed("192.168.0.1".parse().unwrap()));
    }

    #[test]
    fn allowed_cidr_restrictive() {
        let bl = IpBlocklist::new(vec!["10.0.0.0/8".into()], vec![], None, "5m");
        assert!(bl.is_allowed("10.0.0.1".parse().unwrap()));
        assert!(!bl.is_allowed("192.168.0.1".parse().unwrap()));
    }

    #[test]
    fn deny_overrides_allow() {
        let bl = IpBlocklist::new(
            vec!["0.0.0.0/0".into()],
            vec!["10.0.0.0/8".into()],
            None, "5m",
        );
        assert!(!bl.is_allowed("10.0.0.1".parse().unwrap()));
        assert!(bl.is_allowed("192.168.0.1".parse().unwrap()));
    }

    #[test]
    fn auto_block_after_violations() {
        let bl = IpBlocklist::new(vec![], vec![], Some(3), "5m");
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(bl.is_allowed(ip));
        bl.record_violation(ip, 1);
        assert!(bl.is_allowed(ip));
        bl.record_violation(ip, 2);
        assert!(bl.is_allowed(ip));
        bl.record_violation(ip, 3);
        assert!(!bl.is_allowed(ip));
    }
}
