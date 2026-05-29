# Syncara Architecture

## Tech Stack

| Layer         | Choice                                   | Rationale                                                                                                            |
| ------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Language      | Rust (latest stable)                     | Static binary, zero-cost abstractions, `std`-first standard library, strong type system prevents config/routing bugs |
| Async runtime | `tokio` (multi-threaded)                 | De-facto Rust async runtime; multi-threaded worker-stealing scheduler for production proxy workloads                 |
| HTTP handling | `hyper` 1.x (low-level service + client) | Foundation of Rust HTTP ecosystem; gives us full control over the proxy loop without framework overhead              |
| TLS           | `rustls`                                 | Pure Rust TLS — no OpenSSL linkage, static binary stays static                                                       |
| WebSocket     | `tokio-tungstenite`                      | Minimal, correct, async-native WebSocket handling                                                                    |
| Config        | `serde` + `toml`                         | TOML is Rust-native, unambiguous, well-supported by serde; YAML alternative via feature flag in future               |
| Metrics       | `prometheus-client`                      | Official Rust Prometheus client; push-free, scrape-model native                                                      |
| Logging       | `tracing`                                | Structured, async-aware, supports JSON output and span propagation                                                   |
| CLI           | `clap`                                   | De-facto Rust CLI parser; derive macro for minimal boilerplate                                                       |
| Signals       | `tokio::signal`                          | Native async signal handling — no external dependency needed                                                         |

**Dependency budget**: target < 40 direct dependencies. Audit every addition for maintenance status, soundness, and necessity.

---

## 1. High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         syncara binary                              │
│                                                                     │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌────────────────┐  │
│  │  Config   │   │  Config  │   │ Runtime  │   │  Observability │  │
│  │  Loader   │──►│ Watcher  │──►│ Manager  │◄──┤  (tracing +    │  │
│  │  (TOML)   │   │ (SIGHUP) │   │          │   │   metrics)     │  │
│  └──────────┘   └──────────┘   └────┬─────┘   └────────────────┘  │
│                                      │                             │
│  ┌───────────────────────────────────┴──────────────────────────┐  │
│  │                      Listener Pool                           │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                   │  │
│  │  │ TCP      │  │ TLS      │  │ HTTP/2   │  (future)         │  │
│  │  │ Listener │──► Listener │──► Listener │                   │  │
│  │  └──────────┘  └──────────┘  └──────────┘                   │  │
│  └───────────────────────────────────┬──────────────────────────┘  │
│                                      │                             │
│  ┌───────────────────────────────────▼──────────────────────────┐  │
│  │                      ProxyEngine                            │  │
│  │                                                             │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │  │
│  │  │  Router  │──► Balancer │──►  Proxy   │──► Upstream │   │  │
│  │  │(host/path│  │ (strategy)│  │ (forward)│  │ Connection│  │  │
│  │  │  match)  │  │           │  │          │  │   Pool    │   │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │  │
│  │                                                             │  │
│  │  ┌─────────────────────────────────────────────────┐       │  │
│  │  │        WebSocket Detector & Tunneler            │       │  │
│  │  │  (intercepts Upgrade: websocket → raw TCP pipe) │       │  │
│  │  └─────────────────────────────────────────────────┘       │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  Cross-cutting Services                                      │  │
│  │                                                              │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │  │
│  │  │  Health      │  │   Session    │  │  Metrics         │   │  │
│  │  │  Monitor     │  │   Store      │  │  (/metrics       │   │  │
│  │  │  (active +   │  │   (cookie/IP │  │   endpoint)      │   │  │
│  │  │   passive)   │  │    affinity) │  │                  │   │  │
│  │  └──────────────┘  └──────────────┘  └──────────────────┘   │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Runtime Lifecycle

```
  ┌─────────┐
  │  Start  │
  └────┬────┘
       │
  ┌────▼────────┐
  │ Parse CLI   │   clap: --config <path>, --log-level
  └────┬────────┘
       │
  ┌────▼────────┐
  │ Load Config │   serde::Deserialize → Config struct → validate
  └────┬────────┘
       │
  ┌────▼────────────┐
  │ Init Components │   Build all state: Router, Balancers,
  │                 │   HealthMonitor, SessionStore, Metrics
  └────┬────────────┘
       │
  ┌────▼──────────┐
  │  Bind Ports   │   TCP bind (may be privileged → user warns)
  └────┬──────────┘
       │
  ┌────▼────────────┐
  │  Accept Loop    │   tokio::select! over:
  │                 │     • listener.accept()
  │                 │     • signal watcher
  │                 │     • health ticker
  └────┬────────────┘
       │
  ┌────▼──────────┐
  │  Handle Request│   Per-connection async task:
  │                 │   parse → route → balance → proxy
  └────┬──────────┘
       │
  ┌────▼──────────┐
  │   Shutdown     │   SIGTERM/SIGINT → stop accept →
  │                │   drain in-flight (bounded wait) → flush metrics → exit 0
  └────────────────┘
```

### Hot Reload (SIGHUP)

```
SIGHUP ──► ConfigWatcher ──► parse new config ──► validate
           │
           ├── success → swap Arc<Config> atomically
           │              • Router rebuilds routing table
           │              • Balancers reweight upstreams
           │              • HealthMonitor re-registers checks
           │              • Existing connections complete on old config
           │              • Log: "config reloaded" with diff summary
           │
           └── failure → log error, keep running on old config
                          (never crash on bad config at runtime)
```

### On-disk Change Watch (optional, v1.1)

```
inotify/kqueue ──► debounce (200ms) ──► same path as SIGHUP
```

---

## 3. Internal Module Boundaries

### 3.1 `config` — Configuration Loading and Validation

**Responsibility**: Read TOML from file/string, parse into strongly-typed structs, validate semantic rules, produce validated `Config`.

**Key types**:

```
Config
├── listeners: Vec<ListenerConfig>
│   ├── address: SocketAddr
│   ├── tls: Option<TlsConfig>  (cert + key paths)
│   └── routes: Vec<RouteConfig>
│       ├── host: Option<String>    (match Host header)
│       ├── path: Option<String>    (match path prefix)
│       └── upstream_pool: UpstreamPoolConfig
│           ├── strategy: Strategy  (RoundRobin | LeastConnections | IpHash)
│           ├── upstreams: Vec<UpstreamConfig>
│           │   ├── addr: SocketAddr
│           │   ├── weight: u32
│           │   └── health: HealthCheckConfig
│           │       ├── active: ActiveCheck  (tcp | http, interval, timeout)
│           │       └── passive: PassiveCheck (max_fails, fail_timeout)
│           └── session: Option<SessionConfig>
│               └── sticky: StickyConfig
│                   └── mode: Cookie | IpHash
└── admin: AdminConfig
    └── metrics_addr: SocketAddr
```

**Validation rules** (applied after deserialization):

| Rule                                       | Action on failure           |
| ------------------------------------------ | --------------------------- |
| At least 1 listener                        | Hard error, refuse to start |
| At least 1 upstream per pool               | Hard error, refuse to start |
| TLS cert + key both present if tls enabled | Hard error, refuse to start |
| Interval > timeout for active checks       | Hard error, refuse to start |
| addr is valid SocketAddr                   | serde parse error           |
| No duplicate listener addresses            | Hard error, refuse to start |

### 3.2 `proxy` — HTTP Reverse Proxy and WebSocket Tunneling

**Responsibility**: Accept incoming HTTP connections, hand them through the routing pipeline, and forward to upstream. Detect WebSocket upgrades and switch to raw TCP tunnel.

**Key types**:

```
ProxyEngine
├── config: Arc<RwLock<Config>>
├── router: Router
├── metrics: MetricsSink
│
├── async fn accept_loop(listener)        — spawns tasks per connection
├── async fn handle_connection(stream)     — read HTTP, detect WebSocket
│   ├── route_request(req) → Upstream    — delegate to Router + Balancer
│   ├── proxy_http(req, upstream)        — hyper forward
│   └── proxy_websocket(req, upstream)   — tungstenite tunnel
└── async fn drain(timeout)               — graceful shutdown
```

**Data flow per request**:

```
Incoming HTTP request
  │
  ▼
Router::route(&req, config) → matched RouteConfig
  │
  ▼
let upstream = Balancer::select(pool)  → UpstreamAddr (weighted selection)
  │
  ▼
if is_websocket_upgrade(req) {
    proxy_websocket(req, upstream)   — hijack, pipe raw TCP
} else {
    proxy_http(req, upstream)        — hyper::client forward
}
  │
  ▼
record metrics (latency, status, bytes)
```

### 3.3 `routing` — Request Matching and Route Resolution

**Responsibility**: Match incoming requests to route definitions by Host header and path prefix. Return the winning `RouteConfig` and its associated `UpstreamPool`.

**Key types**:

```
Router
├── tables: Vec<RouteTable>     — pre-compiled from Config
│
├── fn route(req) → RouteMatch   — O(n) linear scan; n < 50 for v1
│
RouteMatch {
    pool: Arc<UpstreamPool>,
    params: RouteParams,
}

RouteTable {
    host: Option<String>,        — exact match or wildcard
    path_prefix: Option<String>, — prefix match "/api/" matches "/api/v1/foo"
    pool: Arc<UpstreamPool>,
}
```

**Performance note**: v1 uses linear scan over route table. For typical deployments (5–20 routes), this is sub-microsecond. If route count grows >100, trie-based matching can be added in a future release without public API change.

### 3.4 `balancer` — Upstream Selection Strategies

**Responsibility**: Given an `UpstreamPool` and a request context, select the next upstream to forward to.

**Key types**:

```
trait LoadBalancer: Send + Sync {
    fn select(&self, pool: &UpstreamPool, context: &RequestContext) -> Option<UpstreamAddr>;
}

struct RoundRobin        — atomic counter, modulo upstreams, weighted
struct LeastConnections  — track active connections per upstream, pick lowest
struct IpHash            — hash(client_ip) % upstreams, deterministic per client

UpstreamPool {
    upstreams: Vec<Arc<UpstreamState>>,
    strategy: Strategy,
    balancer: Box<dyn LoadBalancer>,
}

UpstreamState {
    addr: SocketAddr,
    weight: u32,
    active_connections: AtomicU64,   — for least-connections
    healthy: AtomicBool,             — set by HealthMonitor
}
```

### 3.5 `health` — Active and Passive Health Monitoring

**Responsibility**: Track upstream health. Mark upstreams healthy/unhealthy. Remove unhealthy upstreams from rotation. Restore them after recovery.

**Key types**:

```
HealthMonitor
├── checks: Vec<ActiveCheck>   — periodic tasks per upstream
│
├── async fn run()              — main loop: tick every interval
│   └── check(upstream) → bool  — TCP connect or HTTP GET
│       └── on_failure → mark unhealthy
│       └── on_success → mark healthy (if was unhealthy)
│
PassiveHealth
├── track_failure(upstream)     — called by proxy on 5xx / connection error
├── track_success(upstream)     — called by proxy on 2xx
├── ejection: HashMap<UpstreamAddr, EjectionState>
│
EjectionState {
    consecutive_failures: u32,
    ejected_until: Option<Instant>,
}
```

**Health states**:

```
                 ┌──────────┐
     active OK ─►│  Healthy │◄── active OK (was unhealthy)
                 └─────┬────┘
                       │ passive fail (max_fails exceeded)
                       │ OR active fail
                 ┌─────▼──────┐
                 │  Unhealthy  │
                 │ (ejected)   │──► active check OK → Healthy
                 └────────────┘
```

### 3.6 `session` — Sticky Session / Session Affinity

**Responsibility**: Ensure a client is consistently routed to the same upstream for the duration of a session.

**Key types**:

```
trait SessionStore: Send + Sync {
    fn lookup(&self, key: &SessionKey) -> Option<UpstreamAddr>;
    fn set(&self, key: SessionKey, upstream: UpstreamAddr, ttl: Duration);
    fn remove(&self, key: &SessionKey);
}

SessionKey {
    identifier: String,     — cookie value OR "ip:<client_addr>"
}

struct CookieSession {
    cookie_name: String,    — default "_syncara_session"
    store: Arc<dashmap::DashMap<String, SessionEntry>>,
    // in-memory, bounded LRU eviction
}

struct IpHashSession {
    // deterministic via balancer, no store needed
}
```

**Cookie flow**:

```
Request (no cookie)  ──► Balancer picks upstream
                         └── Response Set-Cookie: _syncara_session=<upstream_hash>
                         └── SessionStore::set(hash, upstream)

Request (with cookie)──► SessionStore::lookup(cookie_value)
                         ├── hit → route directly to stored upstream
                         └── miss → Balancer picks new upstream, set cookie
```

### 3.7 `runtime` — Lifecycle Manager

**Responsibility**: Own the top-level orchestration. Start/stop all subsystems. Handle signals. Coordinate graceful shutdown.

**Key types**:

```
RuntimeManager {
    config_watcher: ConfigWatcher,
    proxy: ProxyEngine,
    health_monitor: HealthMonitor,
    metrics_server: MetricsServer,
    shutdown: tokio::sync::watch::Sender<bool>,
}

fn run(config_path) -> Result<()>:
    1. Load config
    2. Init all components
    3. Spawn health_monitor.run()
    4. Spawn metrics_server
    5. Spawn config_watcher (SIGHUP)
    6. Start proxy.accept_loop()
    7. Block on signal:
       - SIGINT / SIGTERM → shutdown()
       - SIGHUP → config_watcher.reload()

async fn shutdown():
    1. Stop accept loop (close listener)
    2. Health monitor → stop
    3. Metrics → flush + stop
    4. Wait for in-flight connections (bounded: 30s max)
    5. Force-remainder abort
    6. Exit 0
```

### 3.8 `observability` — Metrics and Logging

**Responsibility**: Expose Prometheus metrics. Configure structured logging.

**Metrics** (Prometheus):

```
# proxy
syncara_requests_total{listener, upstream, status_class} counter
syncara_requests_active{listener} gauge
syncara_latency_seconds{listener, upstream} histogram
syncara_upstream_health{upstream} gauge (1 = healthy, 0 = unhealthy)
syncara_websocket_upgrades_total{listener} counter

# health
syncara_health_checks_total{upstream, result} counter
syncara_health_check_duration_seconds{upstream} histogram

# runtime
syncara_config_reloads_total{result} counter
syncara_uptime_seconds gauge
```

**Logging**: `tracing` configured to output JSON to stderr. Fields: `timestamp`, `level`, `target`, `message`, structured fields per event. No access log in v1 (can be added as file output later).

---

## 4. Data Flow (Request Lifecycle)

```
Client                         Syncara                          Upstream
  │                              │                                │
  │──── TCP SYN ────────────────►│                                │
  │◄─── SYN ACK ─────────────────│                                │
  │──── TLS handshake ──────────►│                                │
  │◄─── TLS established ────────│                                │
  │                              │                                │
  │──── HTTP GET /api/orders ──►│                                │
  │                              │                                │
  │                              │── Router::route() ──┐          │
  │                              │                      │          │
  │                              │◄── RouteMatch ───────┘          │
  │                              │                                │
  │                              │── Balancer::select() ──┐       │
  │                              │                         │       │
  │                              │◄── Upstream addr ───────┘       │
  │                              │                                │
  │                              │── SessionStore::lookup() ──┐   │
  │                              │                            │   │
  │                              │◄── miss (no cookie) ───────┘   │
  │                              │                                │
  │                              │──── HTTP GET /api/orders ────►│
  │                              │                                │
  │                              │◄─── HTTP 200 + body ──────────│
  │                              │                                │
  │                              │── Set-Cookie: _syncara_...    │
  │                              │── SessionStore::set(...)      │
  │                              │                                │
  │◄─── HTTP 200 + body + cookie─│                                │
  │                              │                                │
  │◄─── Connection close ───────│                                │
  │                              │                                │
  │                              │── record metrics               │
  │                              │── conn_count--                 │
```

### WebSocket Data Flow

```
Client                         Syncara                          Upstream
  │                              │                                │
  │──── HTTP GET /ws ──────────►│                                │
  │    Upgrade: websocket       │                                │
  │    Connection: Upgrade      │                                │
  │                              │                                │
  │                              │── is_websocket_upgrade() = true│
  │                              │── Route + Balance (as above)  │
  │                              │                                │
  │                              │── HTTP to upstream (same req)►│
  │                              │◄── 101 Switching Protocols ───│
  │◄─── 101 Switching Protocols─│                                │
  │                              │                                │
  │═══════ raw TCP tunnel ══════│════════════════════════════════│
  │    (bidirectional pipe)     │                                │
  │═════════════════════════════│════════════════════════════════│
  │                              │                                │
  │                              │── conn_count-- on close       │
```

---

## 5. Responsibility Separation Summary

| Module          | Owns                                            | Does NOT own                          |
| --------------- | ----------------------------------------------- | ------------------------------------- |
| `config`        | Loading, parsing, validation                    | Runtime state, component creation     |
| `proxy`         | Accept loop, HTTP forwarding, WebSocket tunnel  | Routing decisions, health state       |
| `routing`       | Request→route matching, route table             | Connection I/O, upstream selection    |
| `balancer`      | Upstream selection algorithm                    | Route matching, connection management |
| `health`        | Upstream health state, periodic checks          | Traffic routing, metrics collection   |
| `session`       | Session key→upstream mapping, cookie management | Load balancing, health awareness      |
| `runtime`       | Lifecycle, signals, cross-component wiring      | Business logic of any subsystem       |
| `observability` | Metrics registration, logging setup             | Any request-path decision             |

---

## 6. Rust Module Structure

```
syncara/
├── Cargo.toml
├── Cargo.lock
├── README.md                          — short, points to docs/
├── LICENSE
├── docs/                              — design docs, ADRs
│   ├── ARCHITECTURE.md
│   └── SYNCARA_VISION.md
├── config/
│   └── syncara.example.toml           — example config, actively maintained
├── src/
│   ├── main.rs                        — CLI entry: clap parse → RuntimeManager::run()
│   │
│   ├── config/
│   │   ├── mod.rs                     — Config struct, top-level parse + validate
│   │   ├── listener.rs               — ListenerConfig
│   │   ├── upstream.rs               — UpstreamPoolConfig, UpstreamConfig, Strategy enum
│   │   ├── health.rs                 — HealthCheckConfig, ActiveCheck, PassiveCheck
│   │   ├── session.rs                — SessionConfig, StickyConfig
│   │   ├── admin.rs                  — AdminConfig
│   │   └── validate.rs               — ConfigValidator: semantic rules
│   │
│   ├── proxy/
│   │   ├── mod.rs                     — ProxyEngine: accept_loop(), handle_connection()
│   │   ├── http.rs                   — proxy_http(): hyper forward with header manipulation
│   │   └── websocket.rs              — proxy_websocket(): upgrade detection, tunnel setup
│   │
│   ├── routing/
│   │   ├── mod.rs                     — Router, RouteMatch, RouteTable
│   │   └── pattern.rs                — Host/path pattern matching helpers
│   │
│   ├── balancer/
│   │   ├── mod.rs                     — LoadBalancer trait, UpstreamPool
│   │   ├── round_robin.rs            — RoundRobin (weighted, atomic counter)
│   │   ├── least_connections.rs      — LeastConnections (track active conns)
│   │   └── ip_hash.rs                — IpHash (consistent: client_ip % upstreams)
│   │
│   ├── health/
│   │   ├── mod.rs                     — HealthMonitor, run() loop
│   │   ├── active.rs                 — ActiveCheck: TCP connect + optional HTTP probe
│   │   └── passive.rs                — PassiveHealth: failure counting, ejection
│   │
│   ├── session/
│   │   ├── mod.rs                     — SessionStore trait, SessionKey
│   │   ├── cookie.rs                 — CookieSession: dashmap-backed, LRU eviction
│   │   └── ip_hash.rs                — IpHashSession: thin wrapper over IpHash balancer
│   │
│   ├── observability/
│   │   ├── mod.rs                     — init_logging(), init_metrics()
│   │   ├── metrics.rs                — Prometheus metrics registration, /metrics handler
│   │   └── logging.rs                — tracing subscriber setup (JSON, level, file target)
│   │
│   ├── runtime/
│   │   ├── mod.rs                     — RuntimeManager: run(), shutdown()
│   │   └── signals.rs                — Signal handling (SIGTERM, SIGINT, SIGHUP)
│   │
│   └── support/
│       ├── mod.rs                     — Shared utilities
│       └── error.rs                  — Unified error type (thiserror or custom enum)
│
├── tests/
│   ├── integration/
│   │   ├── basic_proxy.rs            — Start syncara, send request, verify response
│   │   ├── health_check.rs           — Start with unhealthy upstream, verify 503
│   │   ├── websocket_test.rs         — WS upgrade and message roundtrip
│   │   ├── sticky_session.rs         — Cookie persistence across requests
│   │   └── config_reload.rs          — SIGHUP → verify routing changes
│   └── fixtures/
│       ├── configs/                   — Test config files
│       └── certs/                     — Test TLS certificates
│
└── benches/
    ├── proxy_throughput.rs            — Requests per second, latency percentiles
    └── balancer_bench.rs              — Balancer selection throughput
```

### Internal dependencies (crate-level):

```
        main
         │
       runtime ─────────────────────┐
       ┌──┬──┬──┬──┬──┐            │
       │  │  │  │  │  │            │
     config proxy health session observability
        │     │                    │
        │   routing                │
        │     │                    │
        │   balancer               │
        │     │                    │
        └──┬──┘                    │
         support ◄─────────────────┘
```

No circular dependencies. `support` contains shared types (`Error`, `UpstreamAddr`, `RequestContext`). All modules depend on `config` (types) and `observability` (logging). No module depends on `runtime` or `main`.

---

## 7. Core Components and Responsibilities

| Component              | File(s)                         | Responsibility                                                         |
| ---------------------- | ------------------------------- | ---------------------------------------------------------------------- |
| `main`                 | `main.rs`                       | Parse CLI args, call `RuntimeManager::run()`, exit with code           |
| `Config`               | `config/mod.rs`                 | Strongly-typed config tree, deserialized from TOML                     |
| `ConfigValidator`      | `config/validate.rs`            | Semantic validation — catches bad config before runtime                |
| `ConfigWatcher`        | `runtime/signals.rs`            | Trap SIGHUP, reload config, atomically swap, log diff                  |
| `ProxyEngine`          | `proxy/mod.rs`                  | Accept loop, per-connection task spawn, delegate to HTTP or WS handler |
| `HttpProxy`            | `proxy/http.rs`                 | Build hyper request to upstream, stream response back, rewrite headers |
| `WebSocketProxy`       | `proxy/websocket.rs`            | Detect WS upgrade, 101 handshake, spawn bidirectional TCP pipe         |
| `Router`               | `routing/mod.rs`                | Match incoming request host/path to route config                       |
| `LoadBalancer` (trait) | `balancer/mod.rs`               | Interface for upstream selection                                       |
| `RoundRobin`           | `balancer/round_robin.rs`       | Weighted round-robin via atomic counter                                |
| `LeastConnections`     | `balancer/least_connections.rs` | Pick upstream with fewest active connections                           |
| `IpHash`               | `balancer/ip_hash.rs`           | Hash client IP, modulo upstream count                                  |
| `HealthMonitor`        | `health/mod.rs`                 | Background loop: tick each upstream, check, update state               |
| `ActiveCheck`          | `health/active.rs`              | TCP connect + optional HTTP `GET /health`                              |
| `PassiveHealth`        | `health/passive.rs`             | Track request failures, eject after threshold                          |
| `SessionStore` (trait) | `session/mod.rs`                | Interface for sticky session storage                                   |
| `CookieSession`        | `session/cookie.rs`             | In-memory map with LRU eviction, cookie-aware                          |
| `MetricsSink`          | `observability/metrics.rs`      | Prometheus counters/histograms, `/metrics` handler                     |
| `RuntimeManager`       | `runtime/mod.rs`                | Orchestrate all components, lifecycle, signals                         |

---

## 8. Future Extensibility Without Overengineering

### Patterns for extensibility (baked in, not bolted on)

1. **Trait-based balancers**: `LoadBalancer` trait means new strategies (e.g., `Random`, `WeightedResponseTime`) are added as new files in `balancer/` with zero changes to the proxy pipeline.

2. **Trait-based session stores**: `SessionStore` trait allows alternative backends (Redis, file-backed) without touching routing or proxy code.

3. **Route table is data, not code**: Routing is 100% config-driven. New match criteria (headers, query params, HTTP method) are added by extending `RouteTable` fields and the matching function — no architectural change.

4. **Middleware hooks (post-v1)**: The `handle_connection()` function is a natural extension point. Before/after hooks can be injected for observability, rate limiting, or request transformation without forking the proxy logic. These are `async fn(Request) -> Result<Request>` closures.

5. **Config is the API**: Adding a new feature means adding a new section to the TOML config. No new subcommands, no new flags, no new daemons.

6. **Metrics are additive**: New metrics are registered at init time. The `/metrics` handler auto-includes any registered metric. No wiring needed.

### Explicitly deferred (non-goals for v1, clean-sheet for later)

- Dynamic upstream API (REST endpoint to add/remove upstreams) — requires `runtime` module to expose an RPC channel, not complex but out of scope for v1.
- Multi-process / worker-per-core — would need shared memory or SO_REUSEPORT; clean separation possible but requires `runtime` changes.
- Plugin system (WASM) — requires trait boundary design in proxy pipeline; straightforward with `Box<dyn Plugin>`, but plugin discovery and sandboxing is non-trivial.
- Configuration sub-key overrides / inheritance — requires config merge logic; defer until configs exceed 100 lines in practice.

### What stays stable (open/closed principle)

| Stable (closed for modification)                              | Extensible (open for extension)                      |
| ------------------------------------------------------------- | ---------------------------------------------------- |
| Request processing pipeline (route → balance → proxy)         | Routing match criteria                               |
| Config format structure (listener → route → upstream)         | New upstream health check types                      |
| Metrics naming convention                                     | New metrics (crates can register additional metrics) |
| Module boundaries (proxy, routing, balancer, health, session) | New balancer strategies, new session backends        |
| Signal handling (SIGHUP, SIGTERM, SIGINT)                     | Config reload listeners (post-reload hooks)          |
| CLI interface (`--config` + `--log-level`)                    | Additional flags (non-breaking)                      |

---

## 9. Key Design Decisions (Concisely)

| Decision          | Choice                                                           | Reasoning                                                                                  |
| ----------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Config format     | TOML                                                             | Rust-native serde support, unambiguous parsing, no footguns with type coercion             |
| HTTP core         | hyper (low-level)                                                | Need control over connection lifecycle for WebSocket and health checks                     |
| TLS               | rustls                                                           | Static binary requirement; no OpenSSL linkage                                              |
| Session storage   | In-memory (dashmap)                                              | No external dependency for v1; LRU bounds memory                                           |
| Health state      | AtomicBool per upstream + tick loop                              | No mutex contention on hot path; eventual consistency is fine                              |
| Hot reload        | Arc<RwLock<Config>> swap                                         | Atomic pointer swap — connections in-flight see old config, new connections see new config |
| Graceful shutdown | track active connections with Arc<AtomicU64>, bound drain at 30s | Bounded wait prevents hang on stuck connections                                            |
| Error handling    | thiserror enum                                                   | Zero-cost, exhaustive matching, no Box<dyn Error> overhead on hot path                     |
| WebSocket         | tokio-tungstenite                                                | Minimal, correct, pure Rust; wraps raw TCP stream                                          |
| Metrics           | prometheus-client crate                                          | Official Rust client; no HTTP framework lock-in                                            |
