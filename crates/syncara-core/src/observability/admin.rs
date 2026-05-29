use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{info, warn};

use super::metrics;

/// Start the admin HTTP server on `addr`.
///
/// Serves:
///   `GET /metrics` — Prometheus text-format metrics dump
///   `GET /status`  — JSON status snapshot
///   `GET /health`  — Always returns `200 OK`
///
/// If `api_key` is set, all endpoints require `Authorization: Bearer <key>`.
///
/// Runs until the shutdown signal fires.
pub async fn serve(addr: SocketAddr, api_key: Option<String>, shutdown_rx: watch::Receiver<bool>) {
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => {
            info!(%addr, "admin server started");
            l
        }
        Err(e) => {
            warn!(%addr, error = %e, "admin server failed to bind");
            return;
        }
    };

    loop {
        let mut shutdown_rx = shutdown_rx.clone();
        let api_key = api_key.clone();

        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _peer)) => {
                        let io = TokioIo::new(stream);
                        tokio::spawn(async move {
                            if let Err(e) = http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| {
                                    handler(req, api_key.clone())
                                }))
                                .await
                            {
                                warn!("admin connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        warn!("admin accept error: {e}");
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                info!("admin server shutting down");
                break;
            }
        }
    }
}

// ── Serialisable JSON shapes for /status ──────────────

#[derive(Serialize)]
struct StatusResponse {
    version: &'static str,
    pools: Vec<PoolJson>,
}

#[derive(Serialize)]
struct PoolJson {
    name: String,
    strategy: String,
    upstreams: Vec<UpstreamJson>,
}

#[derive(Serialize)]
struct UpstreamJson {
    addr: String,
    weight: u32,
    healthy: bool,
    active_connections: u64,
    latency_ms: f64,
}

/// Check if the request is authorised against the configured API key.
fn is_authorised(req: &Request<Incoming>, api_key: &Option<String>) -> bool {
    let Some(ref key) = api_key else {
        return true; // no auth configured
    };
    req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == &format!("Bearer {key}"))
}

/// Single HTTP handler for the admin server.
async fn handler(
    req: Request<Incoming>,
    api_key: Option<String>,
) -> Result<Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    if !is_authorised(&req, &api_key) {
        let body = Full::new(Bytes::from_static(b"unauthorized\n"))
            .map_err(|_: Infallible| unreachable!())
            .boxed();
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("www-authenticate", "Bearer")
            .body(body)
            .expect("static response"));
    }

    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/metrics") => {
            let metric_families = metrics::get().registry.gather();
            let output = prometheus::TextEncoder::new()
                .encode_to_string(&metric_families)
                .unwrap_or_else(|e| format!("metrics encoding error: {e}"));

            let body = Full::new(Bytes::from(output))
                .map_err(|_: Infallible| unreachable!())
                .boxed();

            Ok(Response::builder()
                .header("content-type", "text/plain; charset=utf-8")
                .body(body)
                .expect("static response"))
        }
        (&hyper::Method::GET, "/events") => {
            let rx = match super::realtime::subscribe() {
                Some(rx) => rx,
                None => {
                    let body = Full::new(Bytes::from_static(b"realtime not ready\n"))
                        .map_err(|_: Infallible| unreachable!())
                        .boxed();
                    return Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .body(body)
                        .expect("static response"));
                }
            };

            let stream = super::realtime::SseEventStream::new(rx);
            let body = StreamBody::new(stream)
                .map_err(|_: std::convert::Infallible| -> hyper::Error { unreachable!() })
                .boxed();

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .header("connection", "keep-alive")
                .header("access-control-allow-origin", "*")
                .body(body)
                .expect("static response"))
        }
        (&hyper::Method::GET, "/health") => {
            let body = Full::new(Bytes::from_static(b"ok\n"))
                .map_err(|_: Infallible| unreachable!())
                .boxed();
            Ok(Response::new(body))
        }
        (&hyper::Method::GET, "/status") => {
            let status_guard = crate::runtime::status::get().read().await;
            let snap = &*status_guard;
            let json = serde_json::to_string_pretty(&StatusResponse {
                version: snap.version,
                pools: snap.pools.iter().map(|p| PoolJson {
                    name: p.name.clone(),
                    strategy: p.strategy.clone(),
                    upstreams: p.upstreams.iter().map(|u| UpstreamJson {
                        addr: u.addr.clone(),
                        weight: u.weight,
                        healthy: u.healthy,
                        active_connections: u.active_connections,
                        latency_ms: u.latency_ms,
                    }).collect(),
                }).collect(),
            })
            .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));

            let body = Full::new(Bytes::from(json))
                .map_err(|_: Infallible| unreachable!())
                .boxed();
            Ok(Response::builder()
                .header("content-type", "application/json")
                .body(body)
                .expect("static response"))
        }
        _ => {
            let body = Full::new(Bytes::from_static(b"not found\n"))
                .map_err(|_: Infallible| unreachable!())
                .boxed();
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(body)
                .expect("static response"))
        }
    }
}
