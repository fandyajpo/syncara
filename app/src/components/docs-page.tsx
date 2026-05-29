"use client"

import { useState } from "react"
import { Menu, X, ChevronRight, ExternalLink } from "lucide-react"

const nav = [
  { section: "Getting Started", items: [
    { label: "Installation", href: "#install" },
    { label: "Quickstart", href: "#quickstart" },
    { label: "Zero-Config Mode", href: "#zero-config" },
    { label: "Docker", href: "#docker" },
    { label: "Architecture", href: "#request-flow" },
    { label: "Design Philosophy", href: "#philosophy" },
  ]},
  { section: "Configuration", items: [
    { label: "Full Reference", href: "#config-ref" },
    { label: "Listeners", href: "#config-listeners" },
    { label: "Pools & Upstreams", href: "#config-pools" },
    { label: "Routes", href: "#config-routes" },
    { label: "Security", href: "#config-security" },
    { label: "Logging & Admin", href: "#config-observability" },
    { label: "Validation Rules", href: "#config-validation" },
  ]},
  { section: "Features", groups: [
    { title: "Proxy", items: [
      { label: "Reverse Proxy", href: "#proxy" },
      { label: "TLS Termination", href: "#tls" },
    ]},
    { title: "Traffic", items: [
      { label: "Load Balancing", href: "#balancing" },
      { label: "Health Checks", href: "#health" },
      { label: "WebSocket", href: "#websocket" },
      { label: "Sticky Sessions", href: "#sticky" },
      { label: "Traffic Brain", href: "#brain" },
      { label: "Live Dashboard", href: "/dashboard/brain" },
      { label: "Config Hot Reload", href: "#reload" },
    ]},
    { title: "Security", items: [
      { label: "Rate Limiting", href: "#rate-limit" },
      { label: "IP Blocklisting", href: "#blocklist" },
      { label: "Connection Limits", href: "#conn-limits" },
      { label: "Request Validation", href: "#validation" },
      { label: "Body Size Limit", href: "#body-size" },
      { label: "Timeouts", href: "#timeouts" },
    ]},
  ]},
  { section: "Observability", items: [
    { label: "Admin Server", href: "#admin" },
    { label: "Prometheus Metrics", href: "#metrics" },
    { label: "Logging", href: "#logging" },
    { label: "Status Endpoint", href: "#status-endpoint" },
  ]},
  { section: "CLI", items: [
    { label: "All Commands", href: "#cli" },
    { label: "Global Flags", href: "#cli-global" },
    { label: "start", href: "#cli-start" },
    { label: "init", href: "#cli-init" },
    { label: "validate", href: "#cli-validate" },
    { label: "status", href: "#cli-status" },
    { label: "doctor", href: "#cli-doctor" },
    { label: "tune", href: "#cli-tune" },
    { label: "reload", href: "#cli-reload" },
  ]},
  { section: "Resources", items: [
    { label: "Example Library", href: "#examples" },
    { label: "GitHub", href: "#support" },
  ]},
]

const sections = [
  // ═══════════════════════════════════════════════════════════
  // Getting Started
  // ═══════════════════════════════════════════════════════════
  { id: "install", title: "Installation", content: `
Syncara is a single static binary. No runtime dependencies, no JVM, no Node, no Python.

` + "```sh\ncurl -fsSL https://syncara.sh/install.sh | sh\n```" + `

` + "```sh\nbrew install anomalyco/tap/syncara\n```" + `

` + "```powershell\nirm https://syncara.sh/install.ps1 | iex\n```" + `

Download the binary for your platform from GitHub Releases.

` + "```sh\nsyncara --version\n# → syncara 0.1.0\n```" + `
` },

  { id: "quickstart", title: "Quickstart", content: `
Get a running proxy in 30 seconds.

### 1. Start a backend
` + "```sh\npython3 -m http.server 9001\n```" + `

### 2. Start Syncara
` + "```sh\nsyncara start --backend localhost:9001\n```" + `

### 3. Test
` + "```sh\ncurl http://localhost:8080/\n# → Directory listing from Python server\n```" + `

### 4. Explore
` + "```sh\nsyncara status\ncurl http://127.0.0.1:9090/health\ncurl http://127.0.0.1:9090/metrics\n```" + `
` },

  { id: "zero-config", title: "Zero-Config Mode", content: `
The simplest way to run Syncara — no config file needed.

` + "```sh\nsyncara start --backend localhost:3000\n# Listens on :8080, proxies to :3000\n\n# Custom port\nsyncara start --backend localhost:3000 --port 9000\n```" + `

When you need more control (multiple backends, health checks, routing), create a config file with \`syncara init\` and graduate to config mode.
` },

  { id: "docker", title: "Docker", content: `
Syncara runs in Docker with a minimal image (just the static binary, no runtime deps).

### Build
` + "```dockerfile\nFROM debian:bookworm-slim\nCOPY syncara /usr/local/bin/syncara\nEXPOSE 8080 9090\nENTRYPOINT [\"syncara\"]\nCMD [\"start\"]\n```" + `

` + "```sh\ndocker build -t syncara .\ndocker run -p 8080:8080 -p 9090:9090 \\\n  -v $(pwd)/syncara.yml:/etc/syncara/syncara.yml \\\n  syncara start -c /etc/syncara/syncara.yml\n```" + `

### Zero-config
` + "```sh\ndocker run -p 8080:8080 syncara start --backend host.docker.internal:3000\n```" + `

### Compose
` + "```yaml\nservices:\n  syncara:\n    build: .\n    ports:\n      - \"8080:8080\"\n      - \"9090:9090\"\n    volumes:\n      - ./syncara.yml:/etc/syncara/syncara.yml\n    command: [\"start\", \"-c\", \"/etc/syncara/syncara.yml\"]\n```" + `
` },

  // ═══════════════════════════════════════════════════════════
  // Getting Started (cont.)
  // ═══════════════════════════════════════════════════════════
  { id: "request-flow", title: "Request Flow", content: `
Every request through Syncara passes through four stages:

### 1. Listener
Binds to configured ports (e.g. :8080). Supports plain TCP and TLS termination via rustls. Each accepted connection acquires a global connection permit before any parsing begins.

### 2. Security Layer
Before routing, every request is validated: URI length (< 8 KB), header count (< 128), header value length (< 16 KB), content-length against max body size, malformed Transfer-Encoding detection, and per-IP rate limiting against a sliding window. Blocklisted IPs (CIDR allow/deny) are rejected here. Slow clients are timed out via the request timeout.

### 3. Router
The request is matched against configured routes by host (exact or \`*.example.com\` wildcard) and path prefix. First match wins. Returns a pool name.

### 4. Load Balancer
The pool's strategy selects an upstream. Supported strategies: round-robin, least-connections, weighted, IP hash, sticky (cookie or IP), and brain (deterministic scoring). Per-upstream connection limits are enforced at this stage.

### 5. Upstream
The request is proxied to the selected upstream. Hop-by-hop headers are stripped (RFC 2616 Section 13.5.1). The upstream timeout applies. Response streams back through the same path. Latency is recorded for brain scoring.

### Diagram
` + "```\nClient → Listener → Security (validate, rate-limit, blocklist) → Router → Balancer → Upstream\n                                                    ↓              ↑\n                                               match host+path    health check\n```" + `
` },

  { id: "philosophy", title: "Design Philosophy", content: `
### 1. Every decision is explainable
The Brain balancer logs a full scoring breakdown per request. The operator can always answer "why did that request go there?"

### 2. Configuration is the single source of truth
No runtime API. No in-flight state mutation. The config file at startup defines the entire behavior surface. Hot reload (SIGHUP) re-validates and atomically swaps state.

### 3. Fail closed, not open
Unhealthy upstreams stop receiving traffic immediately. Rate limits return 429. Connection limits reject at the door. Body size limits return 413. Blocked IPs are rejected before any processing.

### 4. Features justify inclusion by production value
Every feature maps to a concrete production scenario. No plugins, no scripting, no platform lock-in.

### 5. Defaults are safe for production
The default config must not crash, leak memory, or fail open. Bounded data structures (rate limiter HashMap capped at 100k entries, latency tracker ring buffers) prevent OOM under attack.
` },

  // ═══════════════════════════════════════════════════════════
  // Configuration
  // ═══════════════════════════════════════════════════════════
  { id: "config-ref", title: "Full Configuration Reference", content: `
Syncara uses a single YAML file. Every field is optional unless marked required.

### Minimal
` + "```yaml\nlisteners:\n  - port: 8080\n\nroutes:\n  - path: /\n    proxy: http://localhost:3000\n\nlogging:\n  level: info\n  format: text\n```" + `

### Complete
` + "```yaml\nlisteners:\n  - port: 8080\n  - port: 8443\n    tls:\n      cert: /etc/certs/cert.pem\n      key: /etc/certs/key.pem\n\npools:\n  - name: web\n    strategy: round-robin\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n        weight: 1\n      - addr: \"10.0.1.85:3000\"\n        weight: 2\n        max_connections: 200\n    connections: 500\n    health:\n      path: /health\n      interval: \"10s\"\n      timeout: \"3s\"\n      unhealthy_threshold: 3\n      healthy_threshold: 2\n    session:\n      enabled: true\n      type: cookie\n      cookie_name: \"_syncara\"\n      ttl: \"1h\"\n    brain:\n      latency_aware: true\n      health_aware: true\n      websocket_pressure_aware: false\n\nroutes:\n  - host: api.example.com\n    path: /\n    pool: web\n  - host: ws.example.com\n    path: /\n    pool: web\n\nsecurity:\n  rate_limit:\n    enabled: true\n    requests_per_minute: 300\n  connections:\n    max_active: 10000\n    websocket_max: 5000\n    per_upstream: 500\n  blocklist:\n    allowed_cidrs:\n      - \"10.0.0.0/8\"\n    denied_cidrs:\n      - \"0.0.0.0/0\"\n    auto_block_after: 5\n    auto_block_ttl: \"5m\"\n  max_body_size: \"10mb\"\n  request_timeout: \"30s\"\n  upstream_timeout: \"30s\"\n  websocket_timeout: \"30m\"\n\nadmin:\n  host: \"127.0.0.1\"\n  port: 9090\n\nlogging:\n  level: info\n  format: json\n```" + `
` },

  { id: "config-listeners", title: "Listeners", content: `
Listeners define the ports Syncara binds to.

| Field | Type | Default | Description |
|---|---|---|---|
| port | u16 | required | TCP port to bind (1-65535) |
| host | string | "0.0.0.0" | Bind address |
| tls.cert | string | null | Path to PEM-encoded TLS certificate |
| tls.key | string | null | Path to PKCS#8 PEM-encoded TLS private key |

TLS is optional per listener. When \`tls.cert\` and \`tls.key\` are both set, Syncara terminates TLS using rustls before handing the connection to the proxy engine.
` },

  { id: "config-pools", title: "Pools & Upstreams", content: `
A pool is a named group of upstream servers with a load-balancing strategy.

### Pool fields

| Field | Type | Default | Description |
|---|---|---|---|
| name | string | "default" | Pool name referenced by routes |
| strategy | string | "round-robin" | One of the strategy values below |
| upstreams | array | [] | List of upstream servers |
| health | object | null | Active health check configuration |
| session | object | null | Sticky session configuration |
| brain | object | null | Brain scoring configuration |
| connections | u32 | null | Per-upstream connection limit (0 = unlimited) |

### Strategy values

| Value | Description |
|---|---|
| round-robin | Distributes evenly across healthy upstreams. Atomic counter modulo healthy count. |
| least-connections | Sends to upstream with fewest active connections. Scans all healthy upstreams. |
| weighted | Distributes by weight ratio using a virtual ring. |
| ip-hash | Same client IP always hits same upstream. FNV-1a hash of IP octets. |
| sticky | Cookie or IP-based session affinity with fallback. |
| brain | Deterministic scoring by health + load + latency (see Traffic Brain section). |

### Upstream fields

| Field | Type | Default | Description |
|---|---|---|---|
| addr | string | required | Upstream address in "host:port" format |
| weight | u32 | 1 | Routing weight for weighted/canary strategies |
| max_connections | u32 | null | Per-upstream connection cap (overrides pool-level \`connections\`) |

### Example: weighted canary (90/10)
` + "```yaml\npools:\n  - name: web\n    strategy: weighted\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n        weight: 90\n      - addr: \"10.0.1.85:3000\"\n        weight: 10\n```" + `
` },

  { id: "config-routes", title: "Routes", content: `
Routes match incoming requests to pools by host header and path prefix. First match wins — order matters.

| Field | Type | Default | Description |
|---|---|---|---|
| host | string | null | Match by Host header (exact or \`*.example.com\` wildcard) |
| path | string | null | Match by path prefix (e.g. \`/api\` matches \`/api/v1\`) |
| pool | string | required | Pool name to route to |
| proxy | string | null | Shorthand: direct upstream URL (creates implicit pool) |

If \`proxy\` is set, Syncara auto-creates a synthetic pool named \`_route_{N}\` with that single upstream. This is expanded during config normalization.

Path matching uses prefix semantics with trailing-slash rules:
- \`/api\` matches \`/api\`, \`/api/\`, \`/api/v1\`, but NOT \`/apiary\`
- \`/api/\` matches \`/api/v1\` but NOT \`/api\`
` },

  { id: "config-security", title: "Security", content: `
All security settings live under the \`security\` key. Every field is optional.

### Rate limiting

| Field | Default | Description |
|---|---|---|
| rate_limit.enabled | false | Enable per-IP sliding-window rate limiting |
| rate_limit.requests_per_minute | 300 | Max requests per IP per minute |

When exceeded, returns HTTP 429. The window is a true sliding window — a burst at minute-start uses quota immediately.

### Connection limits

| Field | Default | Description |
|---|---|---|
| connections.max_active | 10000 | Max concurrent TCP connections (global semaphore) |
| connections.websocket_max | 5000 | Max concurrent WebSocket tunnels |
| connections.per_upstream | null | Per-upstream connection cap (applied in balancer \`acquire()\`) |

### IP blocklisting

| Field | Default | Description |
|---|---|---|
| blocklist.allowed_cidrs | [] | If non-empty, ONLY these CIDRs are permitted |
| blocklist.denied_cidrs | [] | These CIDRs are always rejected (checked before allow) |
| blocklist.auto_block_after | null | Auto-block IP after N rate-limit violations |
| blocklist.auto_block_ttl | "5m" | Auto-block duration |

When \`allowed_cidrs\` is set, the blocklist operates in restrictive mode: only matching IPs are allowed. The \`auto_block_after\` feature integrates with the rate limiter to automatically block abusive IPs.

### Request body size

| Field | Default | Description |
|---|---|---|
| max_body_size | "10mb" | Max request body. Supports "10mb", "500kb", "1gb", or plain bytes |

Requests with a \`content-length\` exceeding this value are rejected with HTTP 413.

### Timeouts

| Field | Default | Description |
|---|---|---|---|
| request_timeout | "30s" | Maximum time to read client request and headers |
| upstream_timeout | "30s" | Maximum time waiting for upstream response |
| websocket_timeout | "30m" | Maximum idle duration for a WebSocket tunnel |
| tcp_keepalive | — | TCP keepalive idle time for upstream connections (e.g. "60s") |

### Example
` + "```yaml\nsecurity:\n  rate_limit:\n    enabled: true\n    requests_per_minute: 300\n  connections:\n    max_active: 10000\n    websocket_max: 5000\n    per_upstream: 500\n  blocklist:\n    denied_cidrs:\n      - \"10.0.0.0/8\"\n  max_body_size: \"10mb\"\n  request_timeout: \"30s\"\n  upstream_timeout: \"30s\"\n  websocket_timeout: \"30m\"\n```" + `
` },

  { id: "config-observability", title: "Logging & Admin", content: `
### Logging

| Field | Type | Default | Allowed |
|---|---|---|---|
| level | string | "info" | trace, debug, info, warn, error |
| format | string | "json" | json, text |

JSON format is recommended for production (can feed into Datadog, Grafana Loki, ELK). Logs go to stderr. The \`RUST_LOG\` env var overrides the configured level.

### Admin server

| Field | Type | Default | Description |
|---|---|---|---|
| admin.host | string | "127.0.0.1" | Admin server bind address |
| admin.port | u16 | 9090 | Admin server port |
| admin.api_key | string | — | Optional Bearer token for admin endpoint auth |
| admin.drain_timeout | string | "5s" | Graceful shutdown drain timeout |

Three endpoints:

| Endpoint | Method | Description |
|---|---|---|
| /health | GET | Returns "ok" — for load balancer health checks |
| /metrics | GET | Prometheus text-format metrics |
| /status | GET | JSON with version, pools, upstream state |
` },

  { id: "config-validation", title: "Validation Rules", content: `
Syncara validates configuration at load time with these rules:

| # | Rule | Error Message |
|---|---|---|
| 1 | At least one listener | \`no listeners defined\` |
| 2 | No duplicate listener host:port | \`listener[{i}] duplicates host:port\` |
| 3 | At least one route | \`no routes defined\` |
| 4 | Each route references an existing pool | \`route[{i}] references pool which is not defined\` |
| 5 | Each pool has >= 1 upstream | \`pool has no upstreams\` |
| 6 | No duplicate upstreams in a pool | \`pool upstream[{j}] duplicates address\` |
| 7 | TLS cert and key files must exist | \`tls.cert not found\` / \`tls.key not found\` |
| 8 | Health check interval > timeout | \`health.interval must be greater than health.timeout\` |
| 9 | Sticky strategy requires session config | \`strategy is 'sticky' but session.enabled is not true\` |
| 10 | Valid session TTL format | \`session.ttl is not valid\` |
| 11 | Session cookie_name not empty | \`session.cookie_name must not be empty\` |
| 12 | Valid log level | Must be trace, debug, info, warn, error |
| 13 | Valid log format | Must be json or text |

CLI output includes friendly hints for common errors (strategy naming, missing fields, address-in-use, EMFILE).
` },

  // ═══════════════════════════════════════════════════════════
  // Features
  // ═══════════════════════════════════════════════════════════
  { id: "proxy", title: "Reverse Proxy", content: `
Syncara proxies HTTP and WebSocket requests to upstream servers.

### How it works
1. Client connects to Syncara on the configured port
2. Security layer validates the request (URI, headers, body size, rate limit, blocklist)
3. Router matches the request to a pool by host + path
4. Balancer selects an upstream from the pool
5. Hop-by-hop headers are stripped (Connection, Transfer-Encoding, Upgrade, etc.)
6. Host header is rewritten to the upstream address
7. Request is forwarded to the upstream with a configurable timeout
8. Response streams back through the same path

### Local testing
` + "```sh\n# Terminal 1 — backend\npython3 -m http.server 9001\n\n# Terminal 2 — syncara\nsyncara start --backend localhost:9001\n\n# Terminal 3 — test\ncurl http://localhost:8080/\n```" + `

### Production layout
` + "```yaml\npools:\n  - name: web\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n      - addr: \"10.0.1.85:3000\"\n\nroutes:\n  - path: /\n    pool: web\n```" + `
` },

  { id: "tls", title: "TLS Termination", content: `
Syncara terminates TLS at the listener level using rustls. TLS is optional per listener.

### Configuration
` + "```yaml\nlisteners:\n  - port: 8443\n    tls:\n      cert: /etc/certs/cert.pem\n      key: /etc/certs/key.pem\n\n  - port: 8080\n  # plain HTTP listener\n```" + `

### Requirements
- Certificate file: PEM-encoded X.509 certificate (may include intermediate CA chain)
- Key file: PKCS#8 PEM-encoded private key
- Both files must exist and be readable at startup

### How it works
When a listener has TLS configured, Syncara builds a \`TlsAcceptor\` from the cert/key pair. Accepted TCP connections are wrapped with TLS before being passed to the HTTP/1.1 service. The rest of the proxy pipeline (routing, balancing, forwarding) is identical between TLS and plaintext listeners.
` },

  { id: "balancing", title: "Load Balancing", content: `
Syncara supports six strategies. Set the strategy per pool.

### round-robin
Distributes evenly across healthy upstreams. Atomic counter modulo healthy count. Skips unhealthy.

Performance: ~147ns/decision at 10 upstreams, ~1µs at 1000.

` + "```yaml\nstrategy: round-robin\n```" + `

### least-connections
Sends to upstream with fewest active connections. Scans all healthy upstreams. Best for workloads with varying request durations (e.g. AI inference, report generation).

` + "```yaml\nstrategy: least-connections\n```" + `

### weighted
Distributes proportionally by weight using a virtual ring (each upstream appears weight/GCD times). Supports canary deploys (90/10, 99/1).

` + "```yaml\nstrategy: weighted\nupstreams:\n  - addr: \"10.0.1.42:3000\"\n    weight: 90\n  - addr: \"10.0.1.85:3000\"\n    weight: 10\n```" + `

### ip-hash
Same client IP always routes to the same upstream. FNV-1a hash of IP octets (IPv4 and IPv6). No cookie needed. Stable as long as healthy upstream set doesn't change.

` + "```yaml\nstrategy: ip-hash\n```" + `

### sticky
Pins a client to the same backend using a cookie (default) or IP. Uses a session store (DashMap with TTL). If the bound upstream becomes unhealthy, reselects via fallback balancer.

` + "```yaml\nstrategy: sticky\nsession:\n  enabled: true\n  type: cookie\n  cookie_name: \"_syncara\"\n  ttl: \"1h\"\n```" + `

### brain
Deterministic scoring engine. See Traffic Brain section.
` },

  { id: "health", title: "Health Checks", content: `
Syncara actively monitors upstream health and stops routing traffic to unhealthy nodes.

### Active health checks
Configured per pool. Each upstream with health configuration gets a background tokio task that periodically probes.

` + "```yaml\npools:\n  - name: web\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n    health:\n      path: /health\n      interval: \"10s\"\n      timeout: \"3s\"\n      unhealthy_threshold: 3\n      healthy_threshold: 2\n```" + `

| Field | Default | Description |
|---|---|---|
| path | "/health" | HTTP path to probe. Empty string = TCP-only check |
| interval | "10s" | Time between probes |
| timeout | "3s" | Per-probe timeout |
| unhealthy_threshold | 2 | Consecutive failures to mark down |
| healthy_threshold | 1 | Consecutive successes to mark up |

Health checks support two modes:
- **TCP**: connects to the upstream port; passes if TCP handshake succeeds within timeout
- **HTTP**: sends GET to the configured path; passes if response is 2xx or 3xx

### How failover works
Each upstream has an \`AtomicBool\` healthy flag. The load balancer filters by this flag on every request. When health checks mark an upstream unhealthy, failover is immediate and automatic as the flag is atomic. A Prometheus gauge (\`syncara_upstream_health\`) and counter (\`syncara_failover_total\`) track state transitions.

### On config reload
Health check tasks are aborted and recreated with new pool configuration.
` },

  { id: "websocket", title: "WebSocket", content: `
Syncara detects WebSocket upgrade requests automatically and tunnels the connection bidirectionally.

### Automatic detection
If a client sends \`Upgrade: websocket\` and \`Connection: Upgrade\`, Syncara detects it and switches to tunneling mode. No special config needed.

### How it works
1. Syncara receives an HTTP request with \`Upgrade: websocket\`
2. A separate WebSocket capacity permit is acquired (in addition to the connection permit)
3. Syncaria connects to the upstream via raw TCP
4. The original request is serialized as HTTP/1.1 bytes and sent to the upstream
5. Syncaria reads the upstream's response (must be 101 Switching Protocols)
6. Hyper's upgrade mechanism hands off the client connection
7. A background task pipes data bidirectionally via \`copy_bidirectional\`
8. The connection stays open until either side closes or the \`websocket_timeout\` fires

### Config
` + "```yaml\npools:\n  - name: ws\n    upstreams:\n      - addr: \"10.0.1.42:9000\"\n\nroutes:\n  - path: /\n    pool: ws\n```" + `

### Timeout
` + "```yaml\nsecurity:\n  websocket_timeout: \"1h\"\n```" + `

### Metrics
- \`syncara_ws_upgrades_total\` — increments per upgrade
- \`syncara_ws_connections_active\` — tracks current tunnels

### Testing
` + "```sh\nwebsocat ws://localhost:8080/\n# → connected to upstream\n```" + `
` },

  { id: "sticky", title: "Sticky Sessions", content: `
Sticky sessions ensure a client always hits the same backend. Useful for WebSocket apps, shopping carts, and dashboards.

### Cookie-based (default)
Syncara sets a cookie (\`_syncara\`) in the response. On subsequent requests, the cookie tells Syncara which backend to use.

` + "```yaml\npools:\n  - name: web\n    strategy: sticky\n    session:\n      enabled: true\n      type: cookie\n      cookie_name: \"_syncara\"\n      ttl: \"1h\"\n```" + `

### IP hash
Stickiness based on client IP. No cookie needed.

` + "```yaml\npools:\n  - name: web\n    strategy: sticky\n    session:\n      enabled: true\n      type: ip-hash\n```" + `

### How it works
1. First request goes to any upstream (via round-robin fallback)
2. Syncara stores the binding: (\`sticky_key → upstream_addr\`) in a DashMap with TTL
3. Subsequent requests with the same key go to the same upstream
4. If the bound upstream is unhealthy, fallback selects another and updates the binding
5. Expired entries are lazily evicted on lookup

### Session stores
- **Cookie**: DashMap keyed by cookie value (extracted from \`Cookie\` header)
- **IpHash**: DashMap keyed by client IP string
` },

  { id: "brain", title: "Traffic Brain", content: `
The Brain is Syncara's deterministic scoring engine for routing decisions. It is NOT AI — every score is calculated from current, measurable signals.

### Scoring signals

Each upstream starts at \`BASE_SCORE = 100\`. Penalties are deducted:

| Signal | Range | Condition |
|---|---|---|
| Health | -100 | Unhealthy upstream (if \`health_aware\`) |
| Active connections | -20 to -40 | Scaled when \`active_conns / weight > 0.8\`, up to -40 at full capacity |
| Latency (p50 > 2x global) | -30 | Per-upstream p50 exceeds 2x global p50 (if \`latency_aware\`) |
| Latency (p50 > 1.5x global) | -25 | Per-upstream p50 exceeds 1.5x global p50 |
| Latency (p50 > 3x global) | -10 | Additional penalty |
| WebSocket pressure | 0 to -15 | Optional (future, requires per-upstream WS counter) |

Score is clamped to minimum 0. Tiebreaker: round-robin among highest-scoring upstreams.

### Configuration
` + "```yaml\npools:\n  - name: web\n    strategy: brain\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n      - addr: \"10.0.1.85:3000\"\n    brain:\n      latency_aware: true\n      health_aware: true\n      websocket_pressure_aware: false\n```" + `

### Decision transparency
Every Brain decision logs a structured breakdown:

` + "```json\n{\"decision\": {\n  \"targets\": [\n    {\"addr\": \"10.0.1.42:3000\", \"score\": 85, \"deductions\": [\"latency: -30\"]},\n    {\"addr\": \"10.0.1.85:3000\", \"score\": 62, \"deductions\": [\"connections: -38\"]}\n  ],\n  \"selected\": \"10.0.1.42:3000\"\n}}\n```" + `

### Latency tracker
The Brain's latency signal comes from a per-upstream ring buffer (size 100). Each proxy response records the elapsed time. The tracker maintains p50 (median) per upstream and a global p50 across all upstreams. The tracker is a singleton initialized at startup.

### When to use Brain
- Latency-sensitive applications (real-time, streaming)
- Heterogeneous backends (different specs, different locations)
- You want to audit why each routing decision was made
` },

  { id: "reload", title: "Config Hot Reload", content: `
Syncara supports hot-reloading configuration without restarting the process. This allows changing routes, pools, strategies, security settings, and timeouts with zero traffic loss.

### Trigger
Send SIGHUP to the process:

` + "```sh\nsyncara reload\n# or manually:\nkill -s HUP $(cat syncara.pid)\n```" + `

### What happens
1. Syncara re-reads and re-validates the config file
2. If validation fails, the old config is kept and an error is logged
3. If valid, new Router, pools, and security layer are built
4. State is atomically swapped via RwLock write locks:
   - Config (routes, listeners, security settings)
   - Router (route matching table)
   - Pools (upstream lists, strategies, health configs)
5. Security layer is re-initialized (new rate limiter, blocklist, connection limits)
6. Old health check tasks are aborted, new ones spawned
7. In-flight requests complete with the old state, new requests use the new state
8. The \`config_reloads_total\` metric is incremented with \`result=success\` or \`result=error\`

### What changes take effect immediately
- Routes (add, remove, reorder)
- Pools and upstreams (add, remove, change strategy)
- Security settings (rate limits, blocklists, timeouts, body size)
- Health check configuration
- Logging level and format

### What requires a restart
- Listeners (ports, TLS certs) — binding new ports requires process restart
` },

  { id: "rate-limit", title: "Rate Limiting", content: `
Syncara uses a per-IP sliding-window rate limiter to protect upstreams from abuse.

### Configuration
` + "```yaml\nsecurity:\n  rate_limit:\n    enabled: true\n    requests_per_minute: 300\n```" + `

### How it works
- Each client IP gets a time-ordered vector of request timestamps
- On each request, timestamps older than 60s are pruned
- If the remaining count exceeds \`requests_per_minute\`, the request gets HTTP 429
- If rate limiting is disabled (default), all requests pass through

### Memory bounds
The rate limiter HashMap is capped at 100,000 entries. When exceeded, the entry with the oldest recent timestamp is evicted. This prevents unbounded memory growth under IP rotation attacks.

### Integration with blocklist
When \`blocklist.auto_block_after\` is configured, the rate limiter tracks violation counts. After N violations within the rate-limit window, the IP is automatically added to the blocklist for \`auto_block_ttl\`.
` },

  { id: "blocklist", title: "IP Blocklisting", content: `
Syncara supports IP blocklisting with CIDR allow/deny rules and automatic blocking of abusive IPs.

### Configuration
` + "```yaml\nsecurity:\n  blocklist:\n    allowed_cidrs:\n      - \"10.0.0.0/8\"\n      - \"192.168.0.0/16\"\n    denied_cidrs:\n      - \"185.220.101.0/24\"\n    auto_block_after: 10\n    auto_block_ttl: \"30m\"\n```" + `

### How it works

**Allow mode**: If \`allowed_cidrs\` is non-empty, the blocklist operates in restrictive mode. Only IPs matching an allowed CIDR are permitted; all others are rejected. This enables running Syncara as an internal-only proxy.

**Deny mode**: IPs matching \`denied_cidrs\` are always rejected. Deny is checked before allow, so a denied CIDR overrides an allow.

**Auto-block**: When \`auto_block_after\` is set, the rate limiter tracks violations per IP. After the threshold is reached, the IP is automatically blocked for \`auto_block_ttl\`. Blocked IPs are stored in a Mutex<HashMap> with expiration; expired entries are cleaned up on each check.

### Order of checks
1. Expired auto-blocks are cleaned up
2. If IP is in the auto-block map, reject
3. If IP matches a denied CIDR, reject
4. If allowed CIDRs are configured and IP doesn't match any, reject
5. Otherwise, allow
` },

  { id: "conn-limits", title: "Connection Limits", content: `
Syncara enforces connection limits at two levels: global and per-upstream.

### Global connection limits
A tokio semaphore limits the total number of concurrent TCP connections. Excess connections are rejected immediately (before any request parsing). A separate semaphore limits WebSocket tunnels.

` + "```yaml\nsecurity:\n  connections:\n    max_active: 10000\n    websocket_max: 5000\n```" + `

- \`max_active\`: Total concurrent connections (default 10,000). Each accepted TCP connection acquires a permit; the permit is held for the connection's lifetime.
- \`websocket_max\`: Max concurrent WebSocket tunnels (default 5,000). A separate pool from the connection semaphore, acquired during WS upgrade.

### Per-upstream connection limits
Each upstream can have a connection cap. When an upstream reaches its limit, the balancer skips it and selects the next healthy upstream.

` + "```yaml\n# Pool-level default for all upstreams in the pool:\npools:\n  - name: web\n    connections: 200\n    upstreams:\n      - addr: \"10.0.1.42:3000\"\n      - addr: \"10.0.1.85:3000\"\n        max_connections: 500  # overrides pool default\n\n# Or via global security config:\nsecurity:\n  connections:\n    per_upstream: 500\n```" + `

The per-upstream limit is checked in \`UpstreamPool::acquire()\` after the balancer selects an upstream. If the selected upstream is at capacity, a fallback scan finds any healthy, non-full upstream.
` },

  { id: "validation", title: "Request Validation", content: `
Syncara validates every incoming request before proxying to protect upstreams from malformed or abusive input.

### Limitations enforced

| Check | Limit | Behavior |
|---|---|---|
| URI length | 8,192 bytes | Returns 414 URI Too Long |
| Header count | 128 headers | Returns 400 Bad Request |
| Header value length | 16,384 bytes per header | Returns 400 Bad Request |
| Transfer-Encoding | Rejects multiple TE values | Returns 400 Bad Request |

### Hop-by-hop header stripping
Per RFC 2616 Section 13.5.1, these headers are removed before forwarding:

\`connection\`, \`keep-alive\`, \`proxy-authenticate\`, \`proxy-authorization\`, \`te\`, \`trailers\`, \`transfer-encoding\`, \`upgrade\`

Additionally, any header named in the \`Connection\` header value is also stripped. For example, if the client sends \`Connection: X-Foo\`, the \`X-Foo\` header is removed before forwarding.
` },

  { id: "body-size", title: "Body Size Limit", content: `
Syncara can reject requests with oversized bodies before forwarding to upstreams.

### Configuration
` + "```yaml\nsecurity:\n  max_body_size: \"10mb\"\n```" + `

### Supported formats
- \`"10mb"\` → 10,485,760 bytes
- \`"500kb"\` → 512,000 bytes
- \`"1gb"\` → 1,073,741,824 bytes
- \`"5242880"\` → plain byte count

### How it works
The \`content-length\` header is checked before proxying. If it exceeds the configured maximum, the request is rejected with HTTP 413 Payload Too Large. Requests without a \`content-length\` header (e.g., chunked encoding without known size) are allowed through and bounded by the \`request_timeout\`.

Default limit: 10 MB.
` },

  { id: "timeouts", title: "Timeouts", content: `
Syncara applies three configurable timeouts to protect against slow clients and upstreams.

| Timeout | Default | Description |
|---|---|---|
| request_timeout | 30s | Maximum time to receive the complete client request. If the client sends headers too slowly or the body is not fully received within this window, the connection is dropped. |
| upstream_timeout | 30s | Maximum time to wait for a response from the upstream after forwarding the request. Returns 504 Gateway Timeout. |
| websocket_timeout | 30m | Maximum idle time for a WebSocket tunnel. If no data flows in either direction for this duration, the tunnel is closed. |

### Configuration
` + "```yaml\nsecurity:\n  request_timeout: \"30s\"\n  upstream_timeout: \"30s\"\n  websocket_timeout: \"30m\"\n```" + `


### How timeouts are applied
- **request_timeout**: wraps the entire \`serve_connection\` future. If the client hasn't completed the HTTP request within this time, the connection is forcibly closed.
- **upstream_timeout**: wraps the \`client.request()\` call. If the upstream hasn't returned a complete response within this time, the proxy returns 504.
- **websocket_timeout**: wraps the \`copy_bidirectional\` future for WebSocket tunnels. A tunnel that's idle for this duration is closed.
` },

  // ═══════════════════════════════════════════════════════════
  // Observability
  // ═══════════════════════════════════════════════════════════

  // ── Admin / Metrics / Logging / Status ────────────────────
  { id: "admin", title: "Admin Server", content: `
The admin server runs on a configurable host:port (default 127.0.0.1:9090) and serves three HTTP endpoints.

### Endpoints

| Endpoint | Method | Description |
|---|---|---|
| /health | GET | Returns "ok" — intended for load balancer health checks |
| /metrics | GET | Prometheus text-format metrics dump |
| /status | GET | JSON snapshot of version, pools, and upstream state |

### Configuration
` + "```yaml\nadmin:\n  host: \"127.0.0.1\"\n  port: 9090\n  # api_key: \"my-secret-token\"\n  # drain_timeout: \"10s\"\n```" + `

Authentication is optional. When \`api_key\` is set, every admin endpoint requires \`Authorization: Bearer <key>\`.

The \`drain_timeout\` controls how long Syncara waits for in-flight requests to complete on shutdown before forcibly exiting (default 5s).

### Testing
` + "```sh\ncurl http://127.0.0.1:9090/health\n# → ok\n\ncurl http://127.0.0.1:9090/metrics\n# → Prometheus text format\n\ncurl http://127.0.0.1:9090/status\n# → JSON\n\n# With API key auth:\ncurl -H 'Authorization: Bearer my-secret-token' http://127.0.0.1:9090/health\n```" + `
` },

  { id: "metrics", title: "Prometheus Metrics", content: `
Available at \`GET /metrics\` on the admin server.

| Metric | Type | Labels | Description |
|---|---|---|---|
| syncara_requests_total | Counter | (none) | Total HTTP requests received |
| syncara_responses_total | CounterVec | status_class (2xx, 4xx, 5xx, error) | Total responses by status class |
| syncara_requests_active | Gauge | (none) | Currently in-flight HTTP requests |
| syncara_latency_seconds | HistogramVec | listener, upstream | Request latency buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s |
| syncara_ws_upgrades_total | Counter | (none) | Total WebSocket upgrade requests |
| syncara_ws_connections_active | Gauge | (none) | Currently active WebSocket tunnels |
| syncara_upstream_health | GaugeVec | upstream | Per-upstream health: 1 = healthy, 0 = unhealthy |
| syncara_failover_total | Counter | (none) | Total upstream failover events |
| syncara_config_reloads_total | CounterVec | result (success, error) | Config reload attempts by outcome |

### Example
` + "```sh\ncurl -s http://127.0.0.1:9090/metrics | grep syncara_requests_total\n# → syncara_requests_total 42\n```" + `
` },

  { id: "logging", title: "Logging", content: `
Syncara logs structured JSON to stderr.

### Configuration
` + "```yaml\nlogging:\n  level: info\n  format: json\n```" + `

### Levels
trace, debug, info, warn, error. The \`RUST_LOG\` env var overrides the config level.

### Formats
- **json**: structured JSON lines. Recommended for production (feed into Datadog, Grafana Loki, ELK).
- **text**: human-readable. Recommended for local development.

### Log fields
- timestamp
- level (trace, debug, info, warn, error)
- target (module path)
- message
- structured fields (method, path, upstream, status, duration_us, etc.)

### Example (JSON format)
` + "```json\n{\"timestamp\":\"...\",\"level\":\"INFO\",\"target\":\"syncara_core::proxy::http\",\"message\":\"proxy request completed\",\"method\":\"GET\",\"path\":\"/api/health\",\"upstream\":\"10.0.1.42:3000\",\"status\":200,\"duration_us\":4123}\n```" + `
` },

  { id: "status-endpoint", title: "Status Endpoint", content: `
\`GET /status\` returns a JSON snapshot of the current runtime state.

### Response shape
` + "```json\n{\n  \"version\": \"0.1.0\",\n  \"pools\": [\n    {\n      \"name\": \"web\",\n      \"strategy\": \"round-robin\",\n      \"upstreams\": [\n        {\n          \"addr\": \"10.0.1.42:3000\",\n          \"weight\": 1,\n          \"healthy\": true,\n          \"active_connections\": 3,\n          \"latency_ms\": 12.5\n        }\n      ]\n    }\n  ]\n}\n```" + `

### Fields
| Field | Description |
|---|---|
| version | Binary version from CARGO_PKG_VERSION |
| pools[].name | Pool name |
| pools[].strategy | Strategy name (round-robin, least-connections, etc.) |
| pools[].upstreams[].addr | Upstream address |
| pools[].upstreams[].healthy | Current health status |
| pools[].upstreams[].active_connections | Current active connection count |
| pools[].upstreams[].latency_ms | Median (p50) latency from the brain tracker |

The status snapshot is read-locked from an in-memory RwLock, safe for concurrent access.
` },

  // ═══════════════════════════════════════════════════════════
  // CLI Reference
  // ═══════════════════════════════════════════════════════════
  { id: "cli", title: "CLI Reference", content: `
Syncara is operated entirely from the command line. Seven subcommands are available.
` },

  { id: "cli-global", title: "Global Flags", content: `
These flags are available on every subcommand:

| Flag | Short | Default | Description |
|---|---|---|---|
| --config | -c | syncara.yml | Config file path |
| --log-level | | info | Log level (trace, debug, info, warn, error) |
| --version | -V | | Print version and exit |
| --help | -h | | Print help and exit |
` },

  { id: "cli-start", title: "start", content: `
Start the Syncara proxy server. This is the default command (runs when no subcommand is given).

` + "```sh\nsyncara start                    # uses syncara.yml\nsyncara start -c /etc/syncara.yml  # custom config path\nsyncara start --backend localhost:3000  # zero-config mode\nsyncara start --backend localhost:3000 --port 9000\n```" + `

| Flag | Default | Description |
|---|---|---|
| --backend | (none) | Upstream address for zero-config mode |
| --port | 8080 | Listen port (only meaningful with --backend) |

With \`--backend\`, Syncara runs in zero-config mode: it generates a minimal config programmatically (one listener, one route, one pool with the given backend). Without \`--backend\`, it reads the config file.

On startup, Syncara writes \`syncara.pid\` with the process ID. This file is used by \`reload\`, \`status\`, and \`doctor\` commands.
` },

  { id: "cli-init", title: "init", content: `
Create a default or example configuration file.

` + "```sh\nsyncara init                        # default config\nsyncara init prod.yml               # custom path\nsyncara init --example sticky-sessions\nsyncara init --example brain\nsyncara init --example websocket\n```" + `

| Argument | Default | Description |
|---|---|---|
| file (positional) | syncara.yml | Output file path |
| --example / -e | (none) | Example template name |

Available examples: hello-world, two-backends, path-routing, host-routing, health-checks, sticky-sessions, websocket, weighted, brain, security, production.

Will not overwrite an existing file.
` },

  { id: "cli-validate", title: "validate", content: `
Load and validate a configuration file. Reports parsed listeners, routes, pools, and upstreams.

` + "```sh\nsyncara validate\nsyncara validate -c /etc/syncara.yml\n```" + `

Shows friendly hints for common errors:
- Strategy naming (underscores → hyphens)
- Missing fields
- EMFILE / too many open files
- Address already in use

On parse errors, shows YAML context around the offending line.
` },

  { id: "cli-status", title: "status", content: `
Fetch live pool/upstream state from a running Syncara process.

` + "```sh\nsyncara status\n# → Syncara Status ─────────────────────\n# Version: 0.1.0\n#   Pool: web (strategy: round-robin)\n#     ✓ localhost:9001  healthy  active: 0  0.0ms\n```" + `

| Flag | Default | Description |
|---|---|---|
| --admin | http://127.0.0.1:9090 | Admin server URL |

Reads from the \`/status\` HTTP endpoint on the admin server.
` },

  { id: "cli-doctor", title: "doctor", content: `
Run diagnostic checks on the configuration and running process.

` + "```sh\nsyncara doctor\n```" + `

Checks performed:
1. **Config validity**: Load and validate the config file
2. **Process status**: Read \`syncara.pid\` and check if the process is alive
3. **Port connectivity**: TCP-probe all configured listener ports
` },

  { id: "cli-tune", title: "tune", content: `
System tuning recommendations and diagnostics.

` + "```sh\nsyncara tune\n```" + `

Reports:
- **System info**: CPU cores, total memory, open file limit (\`ulimit -n\`)
- **Config recommendations**: upstream count, connection limit vs safe values, rate limiting status
- **Network tuning**: \`net.core.somaxconn\`, \`tcp_keepalive_time\` (Linux), file descriptor warnings
` },

  { id: "cli-reload", title: "reload", content: `
Validate the configuration and send SIGHUP to the running process for a hot reload.

` + "```sh\nsyncara reload\n# Checking configuration before reload...\n# ✓  Configuration is valid\n# Sending reload signal to PID 1234...\n# ✓  Reload signal sent to PID 1234\n```" + `

The reload command:
1. Loads and validates the config file
2. Reads the PID from \`syncara.pid\`
3. Sends SIGHUP to the process

The running Syncara process then atomically swaps its configuration (see Config Hot Reload section).
` },

  // ═══════════════════════════════════════════════════════════
  // Examples
  // ═══════════════════════════════════════════════════════════
  { id: "examples", title: "Example Library", content: `
Progressive examples from simple to production-ready. Each is in the \`examples/\` directory with a \`syncara.yml\` + \`README.md\` + test script.

| # | Example | What you learn |
|---|---|---|
| 00 | hello-world | Simplest proxy — one backend, one route |
| 01 | two-backends | Round-robin load balancing |
| 02 | path-routing | Different paths to different backends |
| 03 | host-routing | Virtual hosting by Host header |
| 04 | health-checks | Kill a backend, traffic shifts |
| 05 | sticky-sessions | Cookie-based stickiness |
| 06 | websocket | WebSocket proxying |
| 07 | weighted | 90/10 canary deploy pattern |
| 08 | brain | Latency-aware routing |
| 09 | security | Rate limiting + connection limits + blocklist |
| 10 | production | Full production config with all features |

### Run any example
` + "```sh\ngit clone https://github.com/anomalyco/syncara\ncd syncara\n\n# Start from example config\nsyncara init --example brain\n\n# Or run directly\ncargo run --release -p syncara -- start -c examples/08-brain/syncara.yml\n```" + `
` },

  { id: "support", title: "Support", content: `
Syncara is open source under the MIT License.

- **GitHub:** [github.com/anomalyco/syncara](https://github.com/anomalyco/syncara)
- **Issues:** [github.com/anomalyco/syncara/issues](https://github.com/anomalyco/syncara/issues)
- **Releases:** [github.com/anomalyco/syncara/releases](https://github.com/anomalyco/syncara/releases)
- **License:** MIT
` },
]

export function DocsPage() {
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const [activeId, setActiveId] = useState("#install")

  return (
    <div className="min-h-screen bg-background text-foreground">
      <button
        onClick={() => setSidebarOpen(!sidebarOpen)}
        className="fixed top-4 left-4 z-50 md:hidden p-2 rounded-lg bg-card border border-border"
      >
        {sidebarOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
      </button>

      <header className="border-b border-border bg-card sticky top-0 z-40">
        <div className="max-w-7xl mx-auto px-6 h-14 flex items-center gap-6">
          <a href="/" className="font-bold text-lg hover:text-primary transition-colors flex items-center gap-2">
            <span className="w-6 h-6 rounded bg-primary flex items-center justify-center text-xs font-bold text-primary-foreground">S</span>
            Syncara
          </a>
          <nav className="hidden md:flex items-center gap-1 text-sm">
            <a href="/" className="px-3 py-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary/50 transition-colors">Home</a>
            <a href="/docs" className="px-3 py-1.5 rounded-md text-foreground bg-secondary/50 font-medium transition-colors">Docs</a>
          </nav>
          <div className="ml-auto flex items-center gap-3 text-sm">
            <a
              href="https://github.com/anomalyco/syncara"
              target="_blank"
              rel="noopener noreferrer"
              className="hidden md:inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-border text-muted-foreground hover:text-foreground hover:bg-secondary/50 transition-colors"
            >
              <svg viewBox="0 0 16 16" className="w-4 h-4" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
              GitHub
            </a>
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto flex">
        <aside className={`
          fixed md:sticky top-14 left-0 z-30
          w-64 h-[calc(100vh-3.5rem)] overflow-y-auto
          border-r border-border bg-background
          transition-transform duration-300 ease-in-out
          ${sidebarOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0"}
        `}>
          <nav className="p-3 space-y-5 text-sm">
            {nav.map((group) => (
              <div key={group.section}>
                <div className="font-semibold text-[11px] text-muted-foreground uppercase tracking-widest mb-1.5 px-3">
                  {group.section}
                </div>
                <div className="space-y-px">
                  {group.items?.map((item) => {
                    const isActive = activeId === item.href
                    return (
                      <a
                        key={item.href}
                        href={item.href}
                        onClick={() => { setSidebarOpen(false); setActiveId(item.href) }}
                        className={`flex items-center gap-2 px-3 py-1.5 rounded-md transition-colors text-sm ${
                          isActive
                            ? "text-foreground font-medium bg-secondary/60"
                            : "text-muted-foreground hover:text-foreground hover:bg-secondary/30"
                        }`}
                      >
                        <span className={`w-1 h-1 rounded-full transition-colors ${
                          isActive ? "bg-primary" : "bg-transparent"
                        }`} />
                        {item.label}
                      </a>
                    )
                  })}
                  {group.groups?.map((sub) => (
                    <div key={sub.title} className="mt-2.5">
                      <div className="text-[11px] text-muted-foreground/60 font-medium px-3 mb-0.5 uppercase tracking-wider">
                        {sub.title}
                      </div>
                      {sub.items.map((item) => {
                        const isActive = activeId === item.href
                        return (
                          <a
                            key={item.href}
                            href={item.href}
                            onClick={() => { setSidebarOpen(false); setActiveId(item.href) }}
                            className={`flex items-center gap-2 px-3 py-1 rounded-md transition-colors text-sm ${
                              isActive
                                ? "text-foreground font-medium bg-secondary/60"
                                : "text-muted-foreground hover:text-foreground hover:bg-secondary/30"
                            }`}
                          >
                            <span className={`w-1 h-1 rounded-full transition-colors ${
                              isActive ? "bg-primary" : "bg-transparent"
                            }`} />
                            {item.label}
                          </a>
                        )
                      })}
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </nav>
        </aside>

        {sidebarOpen && (
          <div
            className="fixed inset-0 z-20 bg-black/50 md:hidden"
            onClick={() => setSidebarOpen(false)}
          />
        )}

        <main className="flex-1 min-w-0">
          <div className="max-w-3xl mx-auto px-6 py-12 md:px-10 lg:px-16">
            <div className="space-y-20">
              {sections.map((section, i) => (
                <section
                  key={section.id}
                  id={section.id}
                  className="scroll-mt-24"
                >
                  <h2 className="text-3xl font-bold tracking-tight mb-6 text-foreground">{section.title}</h2>
                  <div className="text-[15px] leading-relaxed text-muted-foreground space-y-4">
                    {renderContent(section.content)}
                  </div>
                  {i < sections.length - 1 && (
                    <div className="mt-16 pt-8 border-t border-border/60">
                      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
                        <div>
                          <p className="text-xs text-muted-foreground/60 mb-1">Next</p>
                          <a
                            href={sections[i + 1]?.id ? `#${sections[i + 1].id}` : "#"}
                            onClick={() => setActiveId(`#${sections[i + 1]?.id}`)}
                            className="text-base text-primary hover:text-primary/80 font-medium transition-colors flex items-center gap-1.5"
                          >
                            {sections[i + 1]?.title}
                            <ChevronRight className="w-4 h-4" />
                          </a>
                        </div>
                        <p className="text-xs text-muted-foreground/40">{i + 2} of {sections.length}</p>
                      </div>
                    </div>
                  )}
                </section>
              ))}
            </div>

            {sections.length > 0 && (
              <footer className="mt-16 pt-8 border-t border-border/40 text-xs text-muted-foreground/50">
                <p>
                  Still need help?{" "}
                  <a href="https://github.com/anomalyco/syncara/issues" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">Open an issue</a>
                  {" "}on GitHub.
                </p>
              </footer>
            )}
          </div>
        </main>
      </div>
    </div>
  )
}

function renderContent(md: string) {
  const lines = md.split("\n")
  const elements: React.ReactNode[] = []
  let inCodeBlock = false
  let codeLines: string[] = []
  let codeLang = ""

  const flushCode = () => {
    if (codeLines.length > 0) {
      const langLabel = codeLang || "text"
      elements.push(
        <div key={`code-${elements.length}`} className="rounded-lg border border-border/50 overflow-hidden mb-4">
          <div className="flex items-center justify-between px-4 py-1.5 bg-secondary/30 border-b border-border/50 text-xs text-muted-foreground font-mono">
            <span>{langLabel}</span>
            <button
              onClick={() => { if (typeof navigator !== "undefined") navigator.clipboard.writeText(codeLines.join("\n")); }}
              className="hover:text-foreground transition-colors"
            >
              Copy
            </button>
          </div>
          <pre className="p-4 overflow-x-auto text-sm font-mono leading-relaxed bg-[#0d1117] m-0">
            <code>{codeLines.map((l, i) => (<span key={i} className="block">{l}</span>))}</code>
          </pre>
        </div>
      )
      codeLines = []
    }
  }

  lines.forEach((line, index) => {
    if (line.startsWith("```") && !inCodeBlock) {
      inCodeBlock = true
      codeLang = line.slice(3).trim()
      return
    }
    if (line.startsWith("```") && inCodeBlock) {
      inCodeBlock = false
      flushCode()
      return
    }
    if (inCodeBlock) {
      codeLines.push(line)
      return
    }
    flushCode()

    if (line.trim() === "") {
      elements.push(<div key={`empty-${index}`} className="h-2" />)
      return
    }

    if (line.startsWith("### ")) {
      elements.push(<h3 key={`h3-${index}`} className="text-lg font-semibold text-foreground mt-8 mb-3 pb-1 border-b border-border/30">{line.slice(4)}</h3>)
      return
    }
    if (line.startsWith("## ")) {
      elements.push(<h3 key={`h2-${index}`} className="text-xl font-semibold text-foreground mt-10 mb-4">{line.slice(3)}</h3>)
      return
    }

    if (line.startsWith("|") && line.endsWith("|")) {
      const cells = line.split("|").filter(c => c.trim())
      if (cells.length > 0 && cells[0].trim().match(/^[-:]+$/)) return

      const isHeader = index > 0 && lines[index - 1].trim() === ""
      if (isHeader || (index > 0 && lines[index - 1].startsWith("|"))) {
        for (let j = elements.length - 1; j >= 0; j--) {
          const el = elements[j] as React.ReactElement | null
          if (el?.type === "table") {
            const tbody = el.props.children[1]
            const rowIndex = (tbody?.props?.children?.length || 0)
            const newRow = (
              <tr key={`tr-${index}`} className={`border-b border-border/40 ${rowIndex % 2 === 0 ? 'bg-secondary/10' : ''}`}>
                {cells.map((c, ci) => (
                  <td key={`td-${ci}`} className="px-4 py-2.5 text-sm">{renderInline(c.trim())}</td>
                ))}
              </tr>
            )
            const updatedTbody = [...(tbody?.props?.children || []), newRow]
            const updatedEl = (
              <table key={`table-${index}`} className="w-full text-sm mb-6 border-collapse border border-border/40 rounded-lg overflow-hidden">
                <thead>{el.props.children[0]}</thead>
                <tbody>{updatedTbody}</tbody>
              </table>
            )
            elements[j] = updatedEl
            return
          }
        }
      }

      elements.push(
        <table key={`table-${index}`} className="w-full text-sm mb-6 border-collapse border border-border/40 rounded-lg overflow-hidden">
          <thead>
            <tr className="border-b border-border/40 bg-secondary/20">
              {cells.map((c, ci) => (
                <th key={`th-${ci}`} className="px-4 py-2.5 text-left font-semibold text-foreground text-xs uppercase tracking-wider">{renderInline(c.trim())}</th>
              ))}
            </tr>
          </thead>
          <tbody />
        </table>
      )
      return
    }

    if (line.trimStart().startsWith("- ")) {
      const text = line.trimStart().slice(2)
      elements.push(
        <li key={`li-${index}`} className="text-muted-foreground ml-4 list-disc">{renderInline(text)}</li>
      )
      return
    }

    if (line.trimStart().match(/^\d+\. /)) {
      const text = line.trimStart().replace(/^\d+\. /, "")
      elements.push(
        <li key={`oli-${index}`} className="text-muted-foreground ml-4 list-decimal">{renderInline(text)}</li>
      )
      return
    }

    elements.push(
      <p key={`p-${index}`} className="text-muted-foreground leading-relaxed">{renderInline(line)}</p>
    )
  })

  flushCode()
  return elements
}

function renderInline(text: string) {
  const parts = text.split(/(`[^`]+`)/g)
  return parts.map((part, i) => {
    if (part.startsWith("`") && part.endsWith("`")) {
      return <code key={i} className="text-xs font-mono bg-secondary/50 px-1.5 py-0.5 rounded border border-border/50">{part.slice(1, -1)}</code>
    }
    const linkParts = part.split(/(\[[^\]]+\]\([^)]+\))/g)
    return linkParts.map((lp, j) => {
      const linkMatch = lp.match(/\[([^\]]+)\]\(([^)]+)\)/)
      if (linkMatch) {
        const href = linkMatch[2]
        const isExternal = href.startsWith("http")
        if (isExternal) {
          return <a key={`${i}-${j}`} href={href} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline inline-flex items-center gap-0.5">{linkMatch[1]} <ExternalLink className="w-3 h-3" /></a>
        }
        return <a key={`${i}-${j}`} href={href} className="text-primary hover:underline">{linkMatch[1]}</a>
      }
      return <span key={`${i}-${j}`}>{lp}</span>
    })
  })
}
