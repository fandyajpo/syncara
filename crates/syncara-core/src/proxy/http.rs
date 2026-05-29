use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{info, warn};

use crate::balancer::{RequestContext, UpstreamPool};
use crate::brain::tracker;
use crate::observability::metrics;
use crate::proxy::websocket;
use crate::routing::Router;

pub fn make_client(keepalive: Option<Duration>) -> Client<HttpConnector, Incoming> {
    let mut connector = HttpConnector::new();
    connector.set_keepalive(keepalive);
    Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector)
}

/// Handle a single accepted TCP connection (plain or TLS).
pub async fn handle_connection<I>(
    io: TokioIo<I>,
    peer_addr: SocketAddr,
    client: Client<HttpConnector, Incoming>,
    router: Arc<tokio::sync::RwLock<Router>>,
    pools: Arc<tokio::sync::RwLock<Vec<UpstreamPool>>>,
) where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let _conn_permit = match crate::security::get().try_acquire_conn() {
        Some(p) => p,
        None => {
            warn!(%peer_addr, "connection rejected — at capacity");
            return;
        }
    };

    let service = service_fn(move |req: Request<Incoming>| {
        let mut client = client.clone();
        let router = router.clone();
        let pools = pools.clone();

        async move {
            proxy_or_error(req, &mut client, &router, &pools, peer_addr).await
        }
    });

    // Slow client detection: wrap serve_connection with request timeout.
    let timeout = crate::security::get().timeouts.request;
    if let Err(e) = tokio::time::timeout(timeout, async {
        http1::Builder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .serve_connection(io, service)
            .await
    })
    .await
    {
        warn!(%peer_addr, error = %e, "connection timed out (slow client)");
    }
}

async fn proxy_or_error(
    req: Request<Incoming>,
    client: &mut Client<HttpConnector, Incoming>,
    router: &Arc<tokio::sync::RwLock<Router>>,
    pools: &Arc<tokio::sync::RwLock<Vec<UpstreamPool>>>,
    peer_addr: SocketAddr,
) -> Result<Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let start = Instant::now();
    let m = metrics::get();
    m.requests_total.inc();
    m.requests_active.inc();

    // ── Security: request validation ───────────────────────────
    if let Err(status) = crate::security::get().validate(&req) {
        warn!(%peer_addr, reason = %status, "request validation failed");
        m.responses_total.with_label_values(&["4xx"]).inc();
        m.requests_active.dec();
        return Ok(error_response(
            status,
            &format!("{} — malformed request", status.as_u16()),
        ));
    }

    // ── Security: body size limit ──────────────────────────────
    if !crate::security::get().check_body_size(req.headers()) {
        m.responses_total.with_label_values(&["4xx"]).inc();
        m.requests_active.dec();
        return Ok(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "413 Payload Too Large — request body exceeds maximum size",
        ));
    }

    // ── Security: rate limiting + blocklist ────────────────────
    if !crate::security::get().check_rate_limit(peer_addr.ip()) {
        warn!(%peer_addr, "rate limit exceeded or IP blocked");
        m.responses_total.with_label_values(&["4xx"]).inc();
        m.requests_active.dec();
        return Ok(error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "429 Too Many Requests — rate limit exceeded",
        ));
    }

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ---------------------------------------------------------------
    // 1. Route
    // ---------------------------------------------------------------
    let route = {
        let router_guard = router.read().await;
        router_guard.route(host.as_deref(), &path)
    };

    let route = match route {
        Some(r) => r,
        None => {
            warn!(%path, "no route matched");
            m.responses_total.with_label_values(&["4xx"]).inc();
            m.requests_active.dec();
            return Ok(error_response(
                StatusCode::NOT_FOUND,
                "404 Not Found — no route matches this request",
            ));
        }
    };

    // ---------------------------------------------------------------
    // 2. Select upstream via load balancer
    // ---------------------------------------------------------------
    let pool = {
        let pools_guard = pools.read().await;
        pools_guard.iter().find(|p| p.name == route.route.pool).cloned()
    };

    let pool = match pool {
        Some(p) => p,
        None => {
            warn!(pool = %route.route.pool, "pool not found");
            m.responses_total.with_label_values(&["5xx"]).inc();
            m.requests_active.dec();
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                "502 Bad Gateway — upstream pool not found",
            ));
        }
    };

    let sticky_key = pool.session_config.as_ref().and_then(|sc| {
        if !sc.enabled {
            return None;
        }
        match sc.sticky_type {
            crate::config::session::StickyType::Cookie => {
                extract_cookie_value(&req, &sc.cookie_name)
            }
            crate::config::session::StickyType::IpHash => {
                Some(peer_addr.ip().to_string())
            }
        }
    });

    let ctx = RequestContext {
        client_ip: peer_addr.ip(),
        sticky_key,
    };

    let (upstream, _guard) = match pool.acquire(&ctx) {
        Some(pair) => pair,
        None => {
            warn!(pool = %pool.name, "all upstreams unhealthy or at capacity");
            m.responses_total.with_label_values(&["5xx"]).inc();
            m.requests_active.dec();
            return Ok(error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "503 Service Unavailable — no healthy upstreams",
            ));
        }
    };

    // ---------------------------------------------------------------
    // 3. WebSocket upgrade check
    // ---------------------------------------------------------------
    if websocket::is_websocket_upgrade(&req) {
        let _ws_permit = match crate::security::get().try_acquire_ws() {
            Some(p) => p,
            None => {
                warn!(%peer_addr, "websocket tunnel rejected — at WS capacity");
                m.responses_total.with_label_values(&["5xx"]).inc();
                m.requests_active.dec();
                return Ok(error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "503 Service Unavailable — WebSocket capacity exhausted",
                ));
            }
        };

        m.ws_upgrades_total.inc();
        info!(%method, %path, %upstream, "websocket upgrade request");
        m.requests_active.dec();
        return websocket::proxy_websocket(req, &upstream, _guard, _ws_permit).await;
    }

    // ---------------------------------------------------------------
    // 4. Build proxied request
    // ---------------------------------------------------------------
    let (mut parts, body) = req.into_parts();

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let upstream_uri: hyper::Uri = format!("http://{}{}", upstream, path_and_query)
        .parse()
        .expect("valid upstream URI");

    parts.uri = upstream_uri;

    // Strip hop-by-hop headers before forwarding.
    crate::security::strip_hop_by_hop(&mut parts.headers);

    // Restore our own host header with the upstream address.
    // (strip_hop_by_hop removes the Connection and hop-by-hop headers,
    // but we also need to set the correct Host for the upstream.)
    if let Ok(host_val) = http::HeaderValue::from_str(&upstream) {
        parts.headers.insert("host", host_val);
    }

    let proxied_req = Request::from_parts(parts, body);

    // ---------------------------------------------------------------
    // 5. Forward with upstream timeout
    // ---------------------------------------------------------------
    let elapsed = start.elapsed();
    let timeout = crate::security::get().timeouts.upstream;

    let result = tokio::time::timeout(timeout, client.request(proxied_req)).await;

    match result {
        Ok(Ok(resp)) => {
            let status = resp.status();
            let status_class = format!("{}xx", status.as_u16() / 100);
            m.responses_total.with_label_values(&[&status_class]).inc();
            m.latency_seconds
                .with_label_values(&["default", &upstream])
                .observe(elapsed.as_secs_f64());
            tracker::get().record(&upstream, elapsed.as_secs_f64());
            m.requests_active.dec();

            info!(
                %method, %path, %upstream, %status,
                duration_us = elapsed.as_micros(),
                "proxy request completed"
            );
            Ok(resp.map(|b| b.boxed()))
        }
        Ok(Err(e)) => {
            warn!(%method, %path, %upstream, error = %e, "upstream request failed");
            m.responses_total.with_label_values(&["error"]).inc();
            m.requests_active.dec();
            Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("502 Bad Gateway — {}", e),
            ))
        }
        Err(_) => {
            warn!(%method, %path, %upstream, timeout_s = ?timeout, "upstream request timed out");
            m.responses_total.with_label_values(&["5xx"]).inc();
            m.requests_active.dec();
            Ok(error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "504 Gateway Timeout — upstream did not respond in time",
            ))
        }
    }
}

fn extract_cookie_value(req: &Request<Incoming>, name: &str) -> Option<String> {
    let cookie_header = req.headers().get("cookie")?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            if key.trim().eq_ignore_ascii_case(name) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn error_response(
    status: StatusCode,
    body: &str,
) -> Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>> {
    let boxed_body = Full::new(Bytes::from(body.to_owned()))
        .map_err(|_: Infallible| unreachable!())
        .boxed();
    Response::builder()
        .status(status)
        .body(boxed_body)
        .expect("static error response is valid")
}
