use hyper::{HeaderMap, Request, StatusCode};

const MAX_URI_LENGTH: usize = 8192;
const MAX_HEADER_COUNT: usize = 128;
const MAX_HEADER_VALUE_LENGTH: usize = 16384;

/// Hop-by-hop headers that MUST NOT be forwarded by a proxy.
/// Per RFC 2616 Section 13.5.1.
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

/// Validate an incoming HTTP request before proxying.
pub fn validate_request<B>(req: &Request<B>) -> Result<(), StatusCode> {
    let uri = req.uri();

    let uri_str = uri.to_string();
    if uri_str.len() > MAX_URI_LENGTH {
        tracing::warn!(uri_len = uri_str.len(), "uri too long");
        return Err(StatusCode::URI_TOO_LONG);
    }

    let header_count = req.headers().len();
    if header_count > MAX_HEADER_COUNT {
        tracing::warn!(%header_count, "too many headers");
        return Err(StatusCode::BAD_REQUEST);
    }

    for (name, value) in req.headers() {
        if value.len() > MAX_HEADER_VALUE_LENGTH {
            tracing::warn!(header = %name, value_len = value.len(), "header value too long");
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    if let Some(te) = req.headers().get("transfer-encoding") {
        if let Ok(val) = te.to_str() {
            if val.to_lowercase().contains("chunked") {
                let parts: Vec<&str> = val.split(',').map(|s| s.trim()).collect();
                if parts.len() > 1 {
                    tracing::warn!(te = %val, "multiple transfer-encoding values");
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
    }

    Ok(())
}

/// Check whether `name` is a hop-by-hop header that should be
/// stripped before forwarding the request to the upstream.
pub fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP.contains(&name.to_ascii_lowercase().as_str())
}

/// Strip hop-by-hop headers from the given `HeaderMap`.
///
/// Removes:
/// 1. All headers listed in `HOP_BY_HOP`
/// 2. Any headers named in the `Connection` header value
///
/// This MUST be called before forwarding a request to an upstream.
pub fn strip_hop_by_hop(headers: &mut HeaderMap) {
    // Collect the names of headers listed in `connection`.
    let mut connection_listed: Vec<String> = Vec::new();
    if let Some(conn_val) = headers.get("connection") {
        if let Ok(val) = conn_val.to_str() {
            for part in val.split(',') {
                let name = part.trim().to_ascii_lowercase();
                if !name.is_empty() {
                    connection_listed.push(name);
                }
            }
        }
    }

    // Remove the `connection` header itself (it's hop-by-hop).
    headers.remove("connection");

    // Remove all known hop-by-hop headers.
    for hbh in HOP_BY_HOP {
        if *hbh == "connection" {
            continue; // already removed
        }
        headers.remove(*hbh);
    }

    // Remove headers listed in the `connection` value.
    for name in &connection_listed {
        if let Ok(hn) = http::HeaderName::from_bytes(name.as_bytes()) {
            headers.remove(hn);
        }
    }
}

/// Check whether the content-length in `headers` exceeds `max_bytes`.
/// Returns `true` (allowed) if no content-length header, if it's
/// within limit, or if parsing fails (safe default: allow).
pub fn check_body_size(headers: &HeaderMap, max_bytes: u64) -> bool {
    if let Some(cl) = headers.get("content-length") {
        if let Ok(val) = cl.to_str() {
            if let Ok(len) = val.parse::<u64>() {
                if len > max_bytes {
                    tracing::warn!(content_length = len, max = max_bytes, "request body exceeds max size");
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;
    use bytes::Bytes;

    fn valid_req() -> Request<Full<Bytes>> {
        Request::builder()
            .uri("/api/health")
            .header("host", "example.com")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    #[test]
    fn valid_request_ok() {
        assert_eq!(validate_request(&valid_req()), Ok(()));
    }

    #[test]
    fn long_uri_rejected() {
        let long_path = "/".repeat(MAX_URI_LENGTH + 1);
        let req = Request::builder()
            .uri(&long_path)
            .header("host", "example.com")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert_eq!(validate_request(&req), Err(StatusCode::URI_TOO_LONG));
    }

    #[test]
    fn too_many_headers_rejected() {
        let mut builder = Request::builder().uri("/").header("host", "x");
        for i in 0..MAX_HEADER_COUNT + 1 {
            builder = builder.header(format!("x-{}", i), "value");
        }
        let req = builder.body(Full::new(Bytes::new())).unwrap();
        assert_eq!(validate_request(&req), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn oversize_header_value_rejected() {
        let long_val = "a".repeat(MAX_HEADER_VALUE_LENGTH + 1);
        let req = Request::builder()
            .uri("/")
            .header("host", "example.com")
            .header("x-custom", &long_val)
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert_eq!(validate_request(&req), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn multiple_transfer_encoding_rejected() {
        let req = Request::builder()
            .uri("/")
            .header("host", "example.com")
            .header("transfer-encoding", "chunked, identity")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert_eq!(validate_request(&req), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn hop_by_hop_detection() {
        assert!(is_hop_by_hop("connection"));
        assert!(is_hop_by_hop("Transfer-Encoding"));
        assert!(is_hop_by_hop("UPGRADE"));
        assert!(!is_hop_by_hop("host"));
        assert!(!is_hop_by_hop("content-type"));
    }

    #[test]
    fn strip_hop_by_hop_removes_standard() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        headers.insert("connection", "close".parse().unwrap());
        headers.insert("transfer-encoding", "chunked".parse().unwrap());
        headers.insert("upgrade", "websocket".parse().unwrap());
        headers.insert("content-type", "text/plain".parse().unwrap());

        strip_hop_by_hop(&mut headers);

        assert!(headers.get("host").is_some());
        assert!(headers.get("content-type").is_some());
        assert!(headers.get("connection").is_none());
        assert!(headers.get("transfer-encoding").is_none());
        assert!(headers.get("upgrade").is_none());
    }

    #[test]
    fn strip_hop_by_hop_removes_connection_listed() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        headers.insert("connection", "x-custom-keep-alive, x-foo".parse().unwrap());
        headers.insert("x-custom-keep-alive", "true".parse().unwrap());
        headers.insert("x-foo", "bar".parse().unwrap());

        strip_hop_by_hop(&mut headers);

        assert!(headers.get("host").is_some());
        assert!(headers.get("connection").is_none());
        assert!(headers.get("x-custom-keep-alive").is_none());
        assert!(headers.get("x-foo").is_none());
    }

    #[test]
    fn body_size_check_blocks_oversized() {
        let mut headers = HeaderMap::new();
        headers.insert("content-length", "10485761".parse().unwrap());
        assert!(!check_body_size(&headers, 10 * 1024 * 1024));
    }

    #[test]
    fn body_size_check_allows_within_limit() {
        let mut headers = HeaderMap::new();
        headers.insert("content-length", "1024".parse().unwrap());
        assert!(check_body_size(&headers, 10 * 1024 * 1024));
    }

    #[test]
    fn body_size_check_allows_no_header() {
        let headers = HeaderMap::new();
        assert!(check_body_size(&headers, 10 * 1024 * 1024));
    }
}
