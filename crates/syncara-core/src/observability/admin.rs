use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{info, warn};

use super::metrics;
use super::manage;
use crate::config::admin::ManagementAuth;

/// Global reference to pools for management UI toggle operations.
static MANAGE_POOLS: std::sync::OnceLock<Arc<tokio::sync::RwLock<Vec<crate::balancer::UpstreamPool>>>> =
    std::sync::OnceLock::new();

/// Global management UI credentials.
static MANAGEMENT_AUTH: std::sync::OnceLock<ManagementAuth> = std::sync::OnceLock::new();

/// Global admin API key (for management route fallback auth).
static ADMIN_API_KEY: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Initialize the management UI with runtime references.
pub fn init_manage(
    pools: Arc<tokio::sync::RwLock<Vec<crate::balancer::UpstreamPool>>>,
    mgmt_auth: Option<ManagementAuth>,
    admin_api_key: Option<String>,
) {
    let _ = MANAGE_POOLS.set(pools);
    manage::init_session_store();
    if let Some(auth) = mgmt_auth {
        let _ = MANAGEMENT_AUTH.set(auth);
    }
    if let Some(key) = admin_api_key {
        let _ = ADMIN_API_KEY.set(key);
    }
}

/// Start the admin HTTP server on `addr`.
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

/// Check if the request is authorised by Bearer token (admin API key).
fn is_authorised(req: &Request<Incoming>, api_key: &Option<String>) -> bool {
    let Some(ref key) = api_key else {
        return true;
    };
    req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == &format!("Bearer {key}"))
}

/// Check if the request has a valid management UI session.
fn has_management_session(req: &Request<Incoming>) -> bool {
    let _ = match MANAGEMENT_AUTH.get() {
        Some(a) => a,
        None => return true, // no management auth configured
    };

    // Check session cookie first
    let cookie_header = req.headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok());
    if let Some(token) = manage::extract_session_token(cookie_header) {
        if manage::validate_session(&token).is_some() {
            return true;
        }
    }

    // Fall back to admin API key Bearer token (for curl/API clients)
    is_authorised_by_api_key(req)
}

/// Check Bearer token against the stored admin API key.
fn is_authorised_by_api_key(req: &Request<Incoming>) -> bool {
    let Some(ref key) = ADMIN_API_KEY.get() else {
        return false; // no admin API key configured
    };
    req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == &format!("Bearer {key}"))
}

fn redirect_to(path: &str) -> Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>> {
    let body = Full::new(Bytes::from_static(b""))
        .map_err(|_: Infallible| unreachable!())
        .boxed();
    Response::builder()
        .status(StatusCode::FOUND)
        .header("location", path)
        .body(body)
        .expect("redirect response")
}

fn html_response(html: String) -> Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>> {
    let body = Full::new(Bytes::from(html))
        .map_err(|_: Infallible| unreachable!())
        .boxed();
    Response::builder()
        .header("content-type", "text/html; charset=utf-8")
        .body(body)
        .expect("static response")
}

fn json_response(body: &str, status: StatusCode) -> Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>> {
    let b = Full::new(Bytes::from(body.to_string()))
        .map_err(|_: Infallible| unreachable!())
        .boxed();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(b)
        .expect("json response")
}

/// Single HTTP handler for the admin server.
async fn handler(
    req: Request<Incoming>,
    api_key: Option<String>,
) -> Result<Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // ── Public routes (no auth required) ────────────────

    // Login page — always accessible
    if method == Method::GET && (path == "/_manage/login" || path == "/_manage/login/") {
        let html = if MANAGEMENT_AUTH.get().is_some() {
            manage::render_login_page(None, None)
        } else {
            manage::render_login_page(None, Some("No authentication configured. Set up management auth in the config file to enable login."))
        };
        return Ok(html_response(html));
    }

    // Login API POST — no auth required
    if method == Method::POST && path == "/_manage/api/login" {
        return Ok(handle_login(req).await);
    }

    // ── Auth check for management routes ────────────────
    let is_manage_page = path.starts_with("/_manage/") && !path.starts_with("/_manage/api/");
    let is_manage_api = path.starts_with("/_manage/api/");

    if is_manage_page || is_manage_api {
        if !has_management_session(&req) {
            if is_manage_api {
                let body = Full::new(Bytes::from_static(b"unauthorized\n"))
                    .map_err(|_: Infallible| unreachable!())
                    .boxed();
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header("www-authenticate", "Bearer")
                    .body(body)
                    .expect("static response"));
            }
            return Ok(redirect_to("/_manage/login"));
        }
    }

    // ── Standard API auth ────────────────────────────────
    if !is_manage_page && !is_manage_api {
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
    }

    // ── Route matching ───────────────────────────────────
    match (method.as_ref(), path.as_str()) {
        ("GET", "/metrics") => {
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
        ("GET", "/events") => {
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
        ("GET", "/health") => {
            let body = Full::new(Bytes::from_static(b"ok\n"))
                .map_err(|_: Infallible| unreachable!())
                .boxed();
            Ok(Response::new(body))
        }
        ("GET", "/status") => {
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

            Ok(json_response(&json, StatusCode::OK))
        }

        // ── Management UI pages ──────────────────────────

        ("GET", "/_manage/" | "/_manage") => {
            let html = handle_manage_overview().await;
            Ok(html_response(html))
        }

        ("GET", "/_manage/upstreams") | ("GET", "/_manage/upstreams/") => {
            let html = handle_manage_upstreams(None).await;
            Ok(html_response(html))
        }

        ("GET", "/_manage/config") | ("GET", "/_manage/config/") => {
            let html = handle_manage_config(None).await;
            Ok(html_response(html))
        }

        // ── Management API ────────────────────────────────

        ("POST", "/_manage/api/upstream/toggle") => {
            let body_bytes = req.collect().await.map(|b| b.to_bytes()).unwrap_or_default();
            let body_str = String::from_utf8_lossy(&body_bytes);
            let html = handle_toggle_upstream(&body_str).await;
            Ok(html_response(html))
        }

        ("POST", "/_manage/api/config/reload") => {
            let html = handle_reload_config().await;
            Ok(html_response(html))
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

// ── Login handler ─────────────────────────────────────

async fn handle_login(
    req: Request<Incoming>,
) -> Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>> {
    let mgmt_auth = match MANAGEMENT_AUTH.get() {
        Some(a) => a,
        None => {
            let html = manage::render_login_page(
                Some("Management authentication is not configured"),
                None,
            );
            return html_response(html);
        }
    };

    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            let html = manage::render_login_page(Some("Failed to read request body"), None);
            return html_response(html);
        }
    };
    let body_str = String::from_utf8_lossy(&body_bytes);

    let username = form_value(&body_str, "username").unwrap_or("");
    let password = form_value(&body_str, "password").unwrap_or("");

    if username == mgmt_auth.username && password == mgmt_auth.password {
        let token = manage::create_session(username);
        let cookie = format!(
            "syncara_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
            token
        );

        let body = Full::new(Bytes::from_static(b""))
            .map_err(|_: Infallible| unreachable!())
            .boxed();
        return Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/_manage/")
            .header("set-cookie", &cookie)
            .body(body)
            .expect("login redirect");
    }

    let html = manage::render_login_page(Some("Invalid username or password"), None);
    html_response(html)
}

// ── Management handlers ───────────────────────────────

async fn handle_manage_overview() -> String {
    let pools = match MANAGE_POOLS.get() {
        Some(p) => manage::collect_pool_info(p),
        None => return manage::render_overview(&[], Some("Management interface not ready")),
    };
    manage::render_overview(&pools, None)
}

async fn handle_manage_upstreams(toast: Option<&str>) -> String {
    let pools = match MANAGE_POOLS.get() {
        Some(p) => manage::collect_pool_info(p),
        None => return manage::render_upstreams(&[], toast),
    };
    manage::render_upstreams(&pools, toast)
}

async fn handle_manage_config(toast: Option<&str>) -> String {
    let config_path = crate::runtime::get_config_path();
    let file_content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => format!("Error reading config file: {e}"),
    };
    manage::render_config(
        &config_path.to_string_lossy(),
        &file_content,
        toast,
    )
}

async fn handle_toggle_upstream(body: &str) -> String {
    let pool_name = form_value(body, "pool");
    let addr = form_value(body, "addr");

    let (pool_name, addr) = match (pool_name, addr) {
        (Some(p), Some(a)) => (p, a),
        _ => {
            let info = manage::collect_pool_info(MANAGE_POOLS.get().unwrap());
            return manage::render_upstreams(
                &info,
                Some("Missing pool or addr parameter"),
            );
        }
    };

    let pools_arc = match MANAGE_POOLS.get() {
        Some(p) => p,
        None => return manage::render_upstreams(&[], Some("Management not ready")),
    };

    let pools = pools_arc.read().await;
    for pool in pools.iter() {
        if pool.name == pool_name {
            for state in pool.upstreams.iter() {
                if state.addr == addr {
                    let current = state.healthy.load(Ordering::Relaxed);
                    let new_health = !current;
                    state.healthy.store(new_health, Ordering::Relaxed);

                    let addr_owned = addr.to_string();
                    let pool_owned = pool_name.to_string();

                    let m = metrics::get();
                    m.upstream_health
                        .with_label_values(&[&addr_owned])
                        .set(if new_health { 1 } else { 0 });
                    if !new_health {
                        m.failover_total.inc();
                    }

                    crate::observability::realtime::broadcast(Arc::new(
                        crate::observability::realtime::RealtimeEvent::HealthTransition(
                            crate::observability::realtime::HealthTransition {
                                addr: addr_owned.clone(),
                                healthy: new_health,
                                pool_name: pool_owned.clone(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            },
                        ),
                    ));

                    let latency = crate::brain::tracker::get()
                        .p50(&addr)
                        .map(|s| s * 1000.0)
                        .unwrap_or(0.0);

                    return manage::render_toggle_row(
                        &pool_name,
                        &addr,
                        state.weight,
                        new_health,
                        state.active_connections.load(Ordering::Relaxed),
                        latency,
                    );
                }
            }
        }
    }

    let info = manage::collect_pool_info(pools_arc);
    manage::render_upstreams(&info, Some(format!("Upstream {addr} not found in pool {pool_name}").as_str()))
}

async fn handle_reload_config() -> String {
    match crate::runtime::trigger_admin_reload() {
        Ok(()) => {
            let info = match MANAGE_POOLS.get() {
                Some(p) => manage::collect_pool_info(p),
                None => vec![],
            };
            manage::render_upstreams(&info, Some("Config reload triggered"))
        }
        Err(e) => {
            let info = match MANAGE_POOLS.get() {
                Some(p) => manage::collect_pool_info(p),
                None => vec![],
            };
            manage::render_upstreams(&info, Some(&format!("Reload error: {e}")))
        }
    }
}

/// Extract a single value from a URL-encoded form body.
fn form_value<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next()?;
        if k == key {
            let v = parts.next()?;
            return Some(v);
        }
    }
    None
}
