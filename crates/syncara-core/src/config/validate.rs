use crate::config::upstream::Strategy;
use crate::config::Config;

/// Run all semantic validation rules against the loaded configuration.
///
/// Returns `Ok(())` if valid, or `Err` with a collected description of all
/// violations found.
pub fn validate_config(cfg: &Config) -> anyhow::Result<()> {
    let mut errors: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // Rule: at least one listener
    // ------------------------------------------------------------------
    if cfg.listeners.is_empty() {
        errors.push("no listeners defined (need at least 1)".into());
    }

    // ------------------------------------------------------------------
    // Rule: no duplicate listener host:port pairs
    // ------------------------------------------------------------------
    {
        let mut seen = std::collections::HashSet::new();
        for (i, l) in cfg.listeners.iter().enumerate() {
            let key = format!("{}:{}", l.host, l.port);
            if !seen.insert(key.clone()) {
                errors.push(format!(
                    "listener[{i}] duplicates host:port \"{key}\""
                ));
            }
        }
    }

    // ------------------------------------------------------------------
    // Rule: at least one route
    // ------------------------------------------------------------------
    if cfg.routes.is_empty() {
        errors.push("no routes defined (need at least 1)".into());
    }

    // ------------------------------------------------------------------
    // Rule: each route references an existing pool
    // ------------------------------------------------------------------
    {
        let pool_names: std::collections::HashSet<&str> =
            cfg.pools.iter().map(|p| p.name.as_str()).collect();

        for (i, route) in cfg.routes.iter().enumerate() {
            if !pool_names.contains(route.pool.as_str()) {
                errors.push(format!(
                    "route[{i}] references pool \"{}\" which is not defined",
                    route.pool
                ));
            }
        }
    }

    // ------------------------------------------------------------------
    // Rule: each pool has at least one upstream
    // ------------------------------------------------------------------
    for pool in &cfg.pools {
        if pool.upstreams.is_empty() {
            errors.push(format!(
                "pool \"{}\" has no upstreams (need at least 1)",
                pool.name
            ));
        }
    }

    // ------------------------------------------------------------------
    // Rule: no duplicate upstream addresses within a pool
    // ------------------------------------------------------------------
    for pool in &cfg.pools {
        let mut seen = std::collections::HashSet::new();
        for (j, u) in pool.upstreams.iter().enumerate() {
            if !seen.insert(u.addr.as_str()) {
                errors.push(format!(
                    "pool \"{}\": upstream[{j}] duplicates address \"{}\"",
                    pool.name, u.addr
                ));
            }
        }
    }

    // ------------------------------------------------------------------
    // Rule: TLS cert + key must both be present, files must exist
    // ------------------------------------------------------------------
    for (i, l) in cfg.listeners.iter().enumerate() {
        if let Some(ref tls) = l.tls {
            if !std::path::Path::new(&tls.cert).exists() {
                errors.push(format!(
                    "listener[{i}]: tls.cert \"{}\" not found",
                    tls.cert
                ));
            }
            if !std::path::Path::new(&tls.key).exists() {
                errors.push(format!(
                    "listener[{i}]: tls.key \"{}\" not found",
                    tls.key
                ));
            }
        }
    }

    // ------------------------------------------------------------------
    // Rule: health check interval must be greater than timeout
    // ------------------------------------------------------------------
    for pool in &cfg.pools {
        if let Some(ref health) = pool.health {
            if let (Ok(interval), Ok(timeout)) =
                (parse_duration(&health.interval), parse_duration(&health.timeout))
            {
                if interval <= timeout {
                    errors.push(format!(
                        "pool \"{}\": health.interval ({}) must be greater than health.timeout ({})",
                        pool.name, health.interval, health.timeout
                    ));
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rule: sticky session config must be present when strategy is Sticky
    // ------------------------------------------------------------------
    for pool in &cfg.pools {
        if matches!(pool.strategy, Strategy::Sticky) {
            match &pool.session {
                Some(s) if s.enabled => {
                    // Validate TTL format
                    if parse_duration(&s.ttl).is_err() {
                        errors.push(format!(
                            "pool \"{}\": session.ttl \"{}\" is not valid (use e.g. \"10s\", \"5m\", \"1h\")",
                            pool.name, s.ttl
                        ));
                    }
                    // Validate cookie_name is not empty
                    if s.cookie_name.is_empty() {
                        errors.push(format!(
                            "pool \"{}\": session.cookie_name must not be empty",
                            pool.name
                        ));
                    }
                }
                _ => {
                    errors.push(format!(
                        "pool \"{}\": strategy is 'sticky' but session.enabled is not true",
                        pool.name
                    ));
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rule: recognized log level
    // ------------------------------------------------------------------
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&cfg.logging.level.as_str()) {
        errors.push(format!(
            "logging.level \"{}\" is not valid (must be one of: {})",
            cfg.logging.level,
            valid_levels.join(", ")
        ));
    }

    // ------------------------------------------------------------------
    // Rule: recognized log format
    // ------------------------------------------------------------------
    let valid_formats = ["json", "text"];
    if !valid_formats.contains(&cfg.logging.format.as_str()) {
        errors.push(format!(
            "logging.format \"{}\" is not valid (must be one of: {})",
            cfg.logging.format,
            valid_formats.join(", ")
        ));
    }

    // ------------------------------------------------------------------
    // Report
    // ------------------------------------------------------------------
    if errors.is_empty() {
        Ok(())
    } else {
        let mut msg = "configuration validation failed:\n".to_string();
        for err in &errors {
            msg.push_str(&format!("  - {err}\n"));
        }
        Err(anyhow::anyhow!(msg))
    }
}

/// Parse a simple duration string (e.g. "10s", "5m", "1h") into seconds.
pub fn parse_duration(s: &str) -> Result<u64, ()> {
    let s = s.trim();
    if s.len() < 2 {
        return Err(());
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str.parse().map_err(|_| ())?;
    match unit {
        "s" => Ok(num),
        "m" => Ok(num * 60),
        "h" => Ok(num * 3600),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("10s"), Ok(10));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("5m"), Ok(300));
    }

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_duration("1h"), Ok(3600));
    }

    #[test]
    fn parse_duration_invalid_unit() {
        assert!(parse_duration("10x").is_err());
    }

    #[test]
    fn parse_duration_no_number() {
        assert!(parse_duration("s").is_err());
    }
}
