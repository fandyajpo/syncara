/// Match a Host header against a pattern.
///
/// Supports exact match (`"example.com"`) and wildcard prefix
/// (`"*.example.com"`). Wildcard matches any single subdomain level.
pub fn match_host(pattern: &str, host: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host.ends_with(suffix)
            && host.len() > suffix.len() + 1
            && host.as_bytes().get(host.len() - suffix.len() - 1) == Some(&b'.')
            && !host[..host.len() - suffix.len() - 1].contains('.')
    } else {
        pattern == host
    }
}

/// Match a request path against a path prefix pattern.
///
/// `pattern` must start with `/`. Matches when `path` starts with
/// `pattern`. A trailing slash is significant:
///   - `"/api"` matches `/api`, `/api/`, `/api/v1`
///   - `"/api/"` matches `/api/`, `/api/v1` but NOT `/api`
pub fn match_path_prefix(pattern: &str, path: &str) -> bool {
    if pattern.is_empty() || path.is_empty() {
        return pattern == path;
    }

    if pattern.ends_with('/') {
        path.starts_with(pattern)
    } else {
        path.starts_with(pattern) && {
            let after = &path[pattern.len()..];
            after.is_empty() || after.starts_with('/')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_host_exact() {
        assert!(match_host("example.com", "example.com"));
        assert!(!match_host("example.com", "www.example.com"));
        assert!(!match_host("example.com", "notexample.com"));
    }

    #[test]
    fn test_match_host_wildcard() {
        assert!(match_host("*.example.com", "www.example.com"));
        assert!(match_host("*.example.com", "api.example.com"));
        assert!(!match_host("*.example.com", "example.com"));
        assert!(!match_host("*.example.com", "deep.www.example.com"));
        assert!(!match_host("*.example.com", "notexample.com"));
    }

    #[test]
    fn test_match_host_wildcard_edge() {
        assert!(!match_host("*.example.com", ".example.com"));
    }

    #[test]
    fn test_match_path_prefix_root() {
        assert!(match_path_prefix("/", "/"));
        assert!(match_path_prefix("/", "/api"));
        assert!(match_path_prefix("/", "/anything"));
    }

    #[test]
    fn test_match_path_prefix_exact() {
        assert!(match_path_prefix("/api", "/api"));
        assert!(match_path_prefix("/api", "/api/"));
        assert!(!match_path_prefix("/api", "/apiary"));
    }

    #[test]
    fn test_match_path_prefix_trailing_slash() {
        assert!(match_path_prefix("/api/", "/api/v1"));
        assert!(match_path_prefix("/api/", "/api/"));
        assert!(!match_path_prefix("/api/", "/api"));
        assert!(!match_path_prefix("/api/", "/apiary"));
    }

    #[test]
    fn test_match_path_prefix_standard() {
        assert!(match_path_prefix("/api/v1", "/api/v1/users"));
        assert!(match_path_prefix("/api/v1", "/api/v1"));
        assert!(!match_path_prefix("/api/v1", "/api/v2"));
    }
}
