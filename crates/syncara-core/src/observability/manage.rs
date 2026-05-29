use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::balancer::UpstreamPool;

// ── Session Store ──────────────────────────────────────

static SESSION_STORE: std::sync::OnceLock<Mutex<SessionStore>> = std::sync::OnceLock::new();

struct SessionStore {
    sessions: HashMap<String, Session>,
    ttl: Duration,
}

struct Session {
    username: String,
    created: Instant,
}

/// Initialise the session store for management UI login.
pub fn init_session_store() {
    let store = SessionStore {
        sessions: HashMap::new(),
        ttl: Duration::from_secs(86400), // 24h
    };
    let _ = SESSION_STORE.set(Mutex::new(store));
}

fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut SessionStore) -> R,
{
    let store = SESSION_STORE
        .get()
        .expect("session store not initialised");
    let mut guard = store.lock().expect("session store lock");
    f(&mut guard)
}

/// Create a new session for the given username.
/// Returns a session token string.
pub fn create_session(username: &str) -> String {
    let token = generate_session_token();
    with_store(|store| {
        store.sessions.insert(
            token.clone(),
            Session {
                username: username.to_string(),
                created: Instant::now(),
            },
        );
        store.evict_expired();
    });
    token
}

/// Validate a session token.
/// Returns `Some(username)` if valid, `None` otherwise.
pub fn validate_session(token: &str) -> Option<String> {
    with_store(|store| {
        store.evict_expired();
        let session = store.sessions.get(token)?;
        if session.created.elapsed() < store.ttl {
            Some(session.username.clone())
        } else {
            store.sessions.remove(token);
            None
        }
    })
}

/// Remove a session (logout).
pub fn destroy_session(token: &str) {
    with_store(|store| {
        store.sessions.remove(token);
    });
}

fn generate_session_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl SessionStore {
    fn evict_expired(&mut self) {
        let ttl = self.ttl;
        self.sessions
            .retain(|_, s| s.created.elapsed() < ttl);
    }
}

/// Data for rendering upstream info in the management UI.
pub struct UpstreamInfo {
    pub addr: String,
    pub weight: u32,
    pub healthy: bool,
    pub active_connections: u64,
    pub latency_ms: f64,
}

/// Data for rendering pool info in the management UI.
pub struct PoolInfo {
    pub name: String,
    pub strategy: String,
    pub upstreams: Vec<UpstreamInfo>,
}

/// Collect a snapshot of pool/upstream state for the management UI.
pub fn collect_pool_info(pools: &Arc<RwLock<Vec<UpstreamPool>>>) -> Vec<PoolInfo> {
    let guard = match pools.try_read() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };

    guard
        .iter()
        .map(|pool| {
            let upstreams = pool
                .upstreams
                .iter()
                .map(|state| {
                    let latency = crate::brain::tracker::get()
                        .p50(&state.addr)
                        .map(|s| s * 1000.0)
                        .unwrap_or(0.0);
                    UpstreamInfo {
                        addr: state.addr.clone(),
                        weight: state.weight,
                        healthy: state.healthy.load(Ordering::Relaxed),
                        active_connections: state.active_connections.load(Ordering::Relaxed),
                        latency_ms: latency,
                    }
                })
                .collect();
            PoolInfo {
                name: pool.name.clone(),
                strategy: strategy_name(&pool.strategy),
                upstreams,
            }
        })
        .collect()
}

fn strategy_name(s: &crate::config::upstream::Strategy) -> String {
    match s {
        crate::config::upstream::Strategy::RoundRobin => "round-robin".into(),
        crate::config::upstream::Strategy::LeastConnections => "least-connections".into(),
        crate::config::upstream::Strategy::Weighted => "weighted".into(),
        crate::config::upstream::Strategy::IpHash => "ip-hash".into(),
        crate::config::upstream::Strategy::Sticky => "sticky".into(),
        crate::config::upstream::Strategy::Brain => "brain".into(),
    }
}

// ── HTML Layout ────────────────────────────────────────

const STYLES: &str = r#"
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
html{font-size:16px}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Oxygen,Ubuntu,sans-serif;background:#0f172a;color:#e2e8f0;line-height:1.5;min-height:100vh}
a{color:#38bdf8;text-decoration:none}
a:hover{text-decoration:underline}
.layout{display:flex;min-height:100vh}
.sidebar{width:240px;background:#1e293b;border-right:1px solid #334155;padding:1.5rem 0;flex-shrink:0}
.sidebar-logo{padding:0 1.5rem 1.5rem;font-size:1.25rem;font-weight:700;color:#f8fafc;border-bottom:1px solid #334155;margin-bottom:1rem;display:flex;align-items:center;gap:.5rem}
.sidebar-logo svg{width:24px;height:24px}
.sidebar-nav{list-style:none;padding:0}
.sidebar-nav li{padding:.5rem 1.5rem;display:flex;align-items:center;gap:.5rem;color:#94a3b8;font-size:.875rem;cursor:pointer;transition:all .15s}
.sidebar-nav li:hover{background:#334155;color:#e2e8f0}
.sidebar-nav li.active{background:#334155;color:#38bdf8;border-right:2px solid #38bdf8}
.sidebar-nav li svg{width:16px;height:16px;flex-shrink:0}
.main{flex:1;padding:2rem;overflow-y:auto}
.page-title{font-size:1.5rem;font-weight:700;margin-bottom:1.5rem;color:#f8fafc}
.card{background:#1e293b;border:1px solid #334155;border-radius:.5rem;margin-bottom:1rem;overflow:hidden}
.card-header{padding:.75rem 1rem;border-bottom:1px solid #334155;font-weight:600;display:flex;align-items:center;justify-content:space-between}
.card-body{padding:1rem}
table{width:100%;border-collapse:collapse}
th{text-align:left;padding:.5rem .75rem;font-size:.75rem;font-weight:600;text-transform:uppercase;letter-spacing:.05em;color:#64748b;border-bottom:1px solid #334155}
td{padding:.5rem .75rem;font-size:.875rem;border-bottom:1px solid #1e293b}
tr:hover td{background:#1a2744}
.health-dot{display:inline-block;width:8px;height:8px;border-radius:50%;margin-right:.5rem}
.health-dot.healthy{background:#22c55e;box-shadow:0 0 4px #22c55e}
.health-dot.unhealthy{background:#ef4444;box-shadow:0 0 4px #ef4444}
.badge{display:inline-block;padding:.125rem .5rem;border-radius:9999px;font-size:.75rem;font-weight:500}
.badge-success{background:#052e16;color:#22c55e;border:1px solid #166534}
.badge-danger{background:#450a0a;color:#ef4444;border:1px solid #7f1d1d}
.badge-neutral{background:#1e293b;color:#94a3b8;border:1px solid #334155}
.badge-info{background:#0c4a6e;color:#38bdf8;border:1px solid #075985}
.btn{display:inline-flex;align-items:center;gap:.25rem;padding:.375rem .75rem;border-radius:.375rem;font-size:.8125rem;font-weight:500;border:1px solid transparent;cursor:pointer;transition:all .15s;line-height:1.5}
.btn:disabled{opacity:.5;cursor:not-allowed}
.btn-sm{padding:.25rem .5rem;font-size:.75rem}
.btn-primary{background:#0284c7;color:#fff;border-color:#0284c7}
.btn-primary:hover:not(:disabled){background:#0369a1}
.btn-danger{background:#dc2626;color:#fff;border-color:#dc2626}
.btn-danger:hover:not(:disabled){background:#b91c1c}
.btn-outline{background:transparent;color:#94a3b8;border-color:#334155}
.btn-outline:hover:not(:disabled){background:#334155;color:#e2e8f0}
.stats-row{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:.75rem;margin-bottom:1.5rem}
.stat-card{background:#1e293b;border:1px solid #334155;border-radius:.5rem;padding:1rem;text-align:center}
.stat-card .stat-value{font-size:1.5rem;font-weight:700;color:#f8fafc}
.stat-card .stat-label{font-size:.75rem;color:#64748b;margin-top:.25rem}
.flash{animation:fadeOut 2s ease-in forwards}
@keyframes fadeOut{0%{opacity:1}70%{opacity:1}100%{opacity:0}}
.toast{position:fixed;top:1rem;right:1rem;background:#1e293b;border:1px solid #334155;border-radius:.5rem;padding:.75rem 1rem;z-index:100;box-shadow:0 4px 12px rgba(0,0,0,.3)}
.toast.success{border-color:#166534}
.toast.error{border-color:#7f1d1d}
.toast .toast-title{font-weight:600;font-size:.875rem}
.toast .toast-detail{font-size:.75rem;color:#94a3b8;margin-top:.25rem}
pre{background:#0f172a;border:1px solid #334155;border-radius:.375rem;padding:1rem;overflow-x:auto;font-size:.8125rem;line-height:1.4;color:#e2e8f0;white-space:pre-wrap;word-break:break-all}
code{font-family:'JetBrains Mono','Fira Code',monospace}
.mb-1{margin-bottom:.5rem}
.mb-2{margin-bottom:1rem}
.mt-2{margin-top:1rem}
.flex{display:flex}
.items-center{align-items:center}
.gap-1{gap:.25rem}
.gap-2{gap:.5rem}
.text-sm{font-size:.875rem}
.text-xs{font-size:.75rem}
.text-muted{color:#64748b}
.truncate{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
#toast-area{position:fixed;top:1rem;right:1rem;z-index:100;display:flex;flex-direction:column;gap:.5rem}
"#;

fn layout(title: &str, active_nav: &str, content: String, toast_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — Syncara Management</title>
<script src="https://unpkg.com/htmx.org@2.0.4"></script>
<style>{styles}</style>
</head>
<body>
<div class="layout">
  <nav class="sidebar">
    <div class="sidebar-logo">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
      Syncara
    </div>
    <ul class="sidebar-nav">
      <li class="{o_nav}" hx-get="/_manage/" hx-target="body" hx-push-url="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
        Overview
      </li>
      <li class="{u_nav}" hx-get="/_manage/upstreams" hx-target="body" hx-push-url="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="8" rx="2"/><rect x="2" y="14" width="20" height="8" rx="2"/><circle cx="6" cy="6" r="1"/><circle cx="6" cy="18" r="1"/></svg>
        Upstreams
      </li>
      <li class="{c_nav}" hx-get="/_manage/config" hx-target="body" hx-push-url="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        Config
      </li>
    </ul>
  </nav>
  <main class="main">
    {content}
  </main>
</div>
<div id="toast-area">{toast_html}</div>
</body>
</html>"#,
        title = title,
        styles = STYLES,
        o_nav = if active_nav == "overview" { "active" } else { "" },
        u_nav = if active_nav == "upstreams" { "active" } else { "" },
        c_nav = if active_nav == "config" { "active" } else { "" },
        content = content,
        toast_html = toast_html,
    )
}

fn toast_success(msg: &str) -> String {
    format!(
        r#"<div class="toast success" id="toast-{}">
  <div class="toast-title">{}</div>
  <div class="toast-detail">Action completed successfully</div>
</div>"#,
        fast_random_id(),
        msg
    )
}

fn fast_random_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("t{}", nanos)
}

// ── Overview Page ──────────────────────────────────────

pub fn render_overview(pools: &[PoolInfo], toast: Option<&str>) -> String {
    let total_upstreams: usize = pools.iter().map(|p| p.upstreams.len()).sum();
    let healthy_upstreams: usize = pools
        .iter()
        .flat_map(|p| p.upstreams.iter())
        .filter(|u| u.healthy)
        .count();
    let total_pools = pools.len();

    let toast_html = toast.map(toast_success).unwrap_or_default();

    let stats = format!(
        r#"<div class="stats-row">
  <div class="stat-card"><div class="stat-value">{}</div><div class="stat-label">Upstreams</div></div>
  <div class="stat-card"><div class="stat-value">{}/{} <span style="font-size:.75rem;color:#64748b">healthy</span></div><div class="stat-label">Health</div></div>
  <div class="stat-card"><div class="stat-value">{}</div><div class="stat-label">Pools</div></div>
</div>"#,
        total_upstreams, healthy_upstreams, total_upstreams, total_pools
    );

    let pools_html: String = pools
        .iter()
        .map(|pool| {
            let upstream_rows: String = pool
                .upstreams
                .iter()
                .map(|u| render_upstream_row(pool.name.as_str(), u))
                .collect();

            format!(
                r#"<div class="card">
  <div class="card-header">
    <span>{name} <span class="badge badge-info">{strategy}</span></span>
    <span class="text-xs text-muted">{count} upstreams</span>
  </div>
  <div class="card-body" style="padding:0">
    <table>
      <thead><tr><th>Status</th><th>Address</th><th>Weight</th><th>Connections</th><th>Latency</th><th>Action</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</div>"#,
                name = pool.name,
                strategy = pool.strategy,
                count = pool.upstreams.len(),
                rows = upstream_rows
            )
        })
        .collect();

    let reload_section = r#"<div class="card">
  <div class="card-header"><span>Config Reload</span></div>
  <div class="card-body flex items-center gap-2">
    <button class="btn btn-outline" hx-post="/_manage/api/config/reload" hx-target="body" hx-swap="outerHTML" hx-confirm="Reload configuration from file?">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
      Reload Config
    </button>
    <span class="text-xs text-muted">Hot-reload configuration from disk without restarting</span>
  </div>
</div>"#;

    let content = format!(
        r#"<h1 class="page-title">Overview</h1>{stats}{reload}{pools}"#,
        stats = stats,
        reload = reload_section,
        pools = pools_html
    );

    layout("Overview", "overview", content, &toast_html)
}

// ── Upstreams Page ─────────────────────────────────────

pub fn render_upstreams(pools: &[PoolInfo], toast: Option<&str>) -> String {
    let toast_html = match toast {
        Some(msg) => toast_success(msg),
        None => String::new(),
    };

    let pools_html: String = pools
        .iter()
        .map(|pool| {
            let upstream_rows: String = pool
                .upstreams
                .iter()
                .map(|u| render_upstream_row(pool.name.as_str(), u))
                .collect();

            format!(
                r#"<div class="card">
  <div class="card-header">
    <span>{name} <span class="badge badge-info">{strategy}</span></span>
    <span class="text-xs text-muted">{count} upstreams</span>
  </div>
  <div class="card-body" style="padding:0">
    <table>
      <thead><tr><th>Status</th><th>Address</th><th>Weight</th><th>Connections</th><th>Latency</th><th>Action</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
</div>"#,
                name = pool.name,
                strategy = pool.strategy,
                count = pool.upstreams.len(),
                rows = upstream_rows
            )
        })
        .collect();

    let content = format!(
        r#"<h1 class="page-title">Upstream Management</h1>
<p class="text-sm text-muted mb-2">Toggle upstream health to take servers in and out of rotation. Note: active health checks may override manual toggles.</p>
{pools}"#,
        pools = pools_html
    );

    layout("Upstreams", "upstreams", content, &toast_html)
}

fn render_upstream_row(pool_name: &str, u: &UpstreamInfo) -> String {
    let health_dot = if u.healthy { "healthy" } else { "unhealthy" };
    let health_label = if u.healthy { "Healthy" } else { "Unhealthy" };
    let badge_class = if u.healthy {
        "badge-success"
    } else {
        "badge-danger"
    };
    let toggle_label = if u.healthy { "Take Down" } else { "Bring Up" };
    let toggle_class = if u.healthy { "btn-danger" } else { "btn-primary" };

    format!(
        r#"<tr>
  <td><span class="health-dot {health_dot}"></span><span class="badge {badge_class}">{health_label}</span></td>
  <td class="text-sm" style="font-family:monospace">{addr}</td>
  <td>{weight}</td>
  <td>{conns}</td>
  <td>{latency}</td>
  <td>
    <button class="btn btn-sm {toggle_class}"
      hx-post="/_manage/api/upstream/toggle"
      hx-vals='{{"pool":"{pool_name}","addr":"{addr}"}}'
      hx-target="closest tr"
      hx-swap="outerHTML">
      {toggle_label}
    </button>
  </td>
</tr>"#,
        health_dot = health_dot,
        badge_class = badge_class,
        health_label = health_label,
        addr = html_escape(&u.addr),
        weight = u.weight,
        conns = u.active_connections,
        latency = format_latency(u.latency_ms),
        toggle_class = toggle_class,
        pool_name = html_escape(pool_name),
        toggle_label = toggle_label,
    )
}

/// Renders a single upstream row after a successful toggle (used as htmx response fragment).
pub fn render_toggle_row(
    pool_name: &str,
    addr: &str,
    weight: u32,
    healthy: bool,
    connections: u64,
    latency_ms: f64,
) -> String {
    let info = UpstreamInfo {
        addr: addr.to_string(),
        weight,
        healthy,
        active_connections: connections,
        latency_ms,
    };
    render_upstream_row(pool_name, &info)
}

// ── Config Page ────────────────────────────────────────

pub fn render_config(config_path: &str, file_content: &str, toast: Option<&str>) -> String {
    let toast_html = match toast {
        Some(msg) => toast_success(msg),
        None => String::new(),
    };

    let content = format!(
        r#"<h1 class="page-title">Configuration</h1>
<div class="card">
  <div class="card-header flex items-center gap-2">
    <span>Config File</span>
    <span class="badge badge-neutral text-xs">{path}</span>
  </div>
  <div class="card-body">
    <pre><code>{config}</code></pre>
  </div>
</div>
<div class="card">
  <div class="card-header"><span>Actions</span></div>
  <div class="card-body flex items-center gap-2">
    <button class="btn btn-primary" hx-post="/_manage/api/config/reload" hx-target="body" hx-swap="outerHTML" hx-confirm="Reload configuration from file?">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
      Reload Config
    </button>
  </div>
</div>"#,
        path = html_escape(config_path),
        config = html_escape(file_content)
    );

    layout("Config", "config", content, &toast_html)
}

// ── Login Page ─────────────────────────────────────────

/// Render the login page.
/// `error` is an optional error message to display.
pub fn render_login_page(error: Option<&str>, message: Option<&str>) -> String {
    let styles = STYLES;
    let error_html = match error {
        Some(msg) => format!(
            r#"<div class="toast error" style="position:static;margin-bottom:1rem">{}</div>"#,
            html_escape(msg)
        ),
        None => String::new(),
    };
    let message_html = match message {
        Some(msg) => format!(
            r#"<div class="toast success" style="position:static;margin-bottom:1rem">{}</div>"#,
            html_escape(msg)
        ),
        None => String::new(),
    };

    let tmpl = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Login — Syncara Management</title>
<script src="https://unpkg.com/htmx.org@2.0.4"></script>
<style>{{styles}}</style>
<style>
.login-page{{display:flex;align-items:center;justify-content:center;min-height:100vh;padding:2rem}}
.login-card{{background:#1e293b;border:1px solid #334155;border-radius:.5rem;padding:2rem;width:100%;max-width:400px}}
.login-card h1{{font-size:1.25rem;font-weight:700;margin-bottom:.25rem;color:#f8fafc}}
.login-card p{{font-size:.875rem;color:#64748b;margin-bottom:1.5rem}}
.form-group{{margin-bottom:1rem}}
.form-group label{{display:block;font-size:.8125rem;font-weight:500;color:#94a3b8;margin-bottom:.375rem}}
.form-group input{{width:100%;padding:.5rem .75rem;background:#0f172a;border:1px solid #334155;border-radius:.375rem;color:#e2e8f0;font-size:.875rem;outline:none;transition:border-color .15s;box-sizing:border-box}}
.form-group input:focus{{border-color:#38bdf8}}
.form-group input::placeholder{{color:#475569}}
.btn-block{{width:100%;justify-content:center}}
.mt-2{{margin-top:.5rem}}
.text-center{{text-align:center}}
</style>
</head>
<body class="login-page">
  <div class="login-card">
    <div style="display:flex;align-items:center;gap:.5rem;margin-bottom:1.5rem">
      <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="#38bdf8" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
      <span style="font-size:1.25rem;font-weight:700;color:#f8fafc">Syncara</span>
    </div>
    {error_html}
    {message_html}
    <form hx-post="/_manage/api/login" hx-target="body" hx-swap="outerHTML" hx-push-url="true">
      <div class="form-group">
        <label for="username">Username</label>
        <input type="text" id="username" name="username" placeholder="Enter username" required autocomplete="username" autofocus>
      </div>
      <div class="form-group">
        <label for="password">Password</label>
        <input type="password" id="password" name="password" placeholder="Enter password" required autocomplete="current-password">
      </div>
      <button type="submit" class="btn btn-primary btn-block mt-2">Sign In</button>
    </form>
  </div>
</body>
</html>"##;
    tmpl.replace("{styles}", styles)
        .replace("{error_html}", &error_html)
        .replace("{message_html}", &message_html)
}

/// Extract a session token from the Cookie header.
pub fn extract_session_token(cookie_header: Option<&str>) -> Option<String> {
    let cookie_str = cookie_header?;
    for part in cookie_str.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("syncara_session=") {
            return Some(value.to_string());
        }
    }
    None
}

// ── Helpers ────────────────────────────────────────────

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_latency(ms: f64) -> String {
    if ms < 0.001 {
        "<1µs".into()
    } else if ms < 1.0 {
        format!("{:.1}µs", ms * 1000.0)
    } else if ms < 1000.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}
