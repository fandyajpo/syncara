# Syncara Vision

## 1. Product Definition

Syncara is a **standalone, self-hosted traffic brain** — a reverse proxy + load balancer that routes traffic based on real-time context (health, connection behavior, session affinity), not just round-robin or random distribution.

It is a single binary with a file-based config. No external dependencies at runtime. Deploy it alongside your application, point traffic at it, and it mediates between clients and upstream servers with awareness of who and what is connecting.

---

## 2. v1 Scope

- Single-node reverse proxy
- Config-driven (YAML/TOML file)
- HTTP/1.1 and HTTP/2 termination
- WebSocket upgrade passthrough (no termination)
- Multiple load balancing strategies: round-robin, least-connections, IP-hash
- Passive and active health checks
- Sticky sessions via cookie or IP affinity
- Prometheus-compatible metrics endpoint
- Graceful shutdown and hot-reload of config (SIGHUP)
- Runs on Linux, macOS (primary tier)

---

## 3. Non-Goals (v1)

| Out of scope | Rationale |
|---|---|
| Service mesh / sidecar injection | Adds orchestration complexity, not needed for single-node deployments |
| Kubernetes CRD / controller | Tight coupling to K8s; config files are sufficient for v1 |
| AI/ML routing | Would require training data, model serving, latency budget — disproportionate value for v1 |
| Multi-region / global anycast | Infrastructure concern, not proxy concern |
| gRPC / HTTP/3 (QUIC) | Can be added later; not required for v1 MVP |
| mTLS / certificate management | Delegate to a reverse TLS terminator (e.g., envoy, nginx) or edge gateway |
| Distributed control plane / gossip protocol | Single binary, single process — no need for peer discovery yet |
| Web UI / dashboard | CLI + metrics endpoint is sufficient; GUI is a v2+ concern |
| Plugin / WASM extensibility | Premature abstraction; define clear internal interfaces first |
| API gateway features (rate limiting, auth, API keys) | Out of scope for a traffic brain; upstream applications handle auth |

---

## 4. Core Engineering Principles

1. **Spawn one process, configure one file, it works** — zero runtime deps, static binary.
2. **Fail closed** — if health checks cannot reach upstreams, return 503. No silent blackholing.
3. **Observable by default** — structured logs (json), metrics on `/metrics`, minimal but useful.
4. **Graceful under pressure** — connection draining on shutdown, backpressure-aware buffering, bounded goroutine pools.
5. **Predictable routing** — same config produces same behavior every time. No surprises.
6. **Simple config, not complex config** — if a setting needs three paragraphs of docs, simplify the design.
7. **Test the data path** — integration tests with real TCP connections, not just unit mocks.

---

## 5. Recommended Architecture Direction

```
                          ┌─────────────┐
  client ──► TCP/TLS ──►  │  Listener   │
                          └──────┬──────┘
                                 │
                          ┌──────▼──────┐
                          │   Router    │  (host/path matching → upstream pool)
                          └──────┬──────┘
                                 │
                          ┌──────▼──────┐
                          │  Balancer   │  (strategy: RR, LC, IP-hash)
                          └──────┬──────┘
                                 │
                          ┌──────▼──────┐
                          │  Proxy      │  (reverse proxy + WebSocket hook)
                          └──────┬──────┘
                                 │
                          ┌──────▼──────┐
                          │  Upstreams  │
                          └─────────────┘

  Cross-cutting:
    • HealthChecker (goroutine loop, marks upstreams up/down)
    • MetricsSink (Prometheus counters/histograms)
    • ConfigWatcher (file watch / SIGHUP → hot reload)
```

Recommended language: **Go 1.22+**.

Rationale: Excellent standard library (`net/http/httputil.ReverseProxy`, `net/http`, `crypto/tls`), static binary, goroutine-per-connection model, first-class Prometheus client, and mature WebSocket libraries.

Key internal packages:

```
internal/
  config/      — parse, validate, hot-reload
  router/      — host/path matching → upstream selection
  balancer/    — strategy implementations
  proxy/       — reverse proxy, WebSocket detection & hijack
  health/      — active & passive health check loop
  metrics/     — Prometheus instrumentation
  server/      — lifecycle, graceful shutdown, signal handling
```

---

## 6. Minimal Feature Set for MVP

| Area | Feature |
|---|---|
| **Core** | HTTP reverse proxy (1 upstream, basic round-robin) |
| **Config** | YAML file with listener, upstream, and health check blocks |
| **Health** | Passive (503 tracking per upstream) + active (periodic TCP/HTTP check) |
| **Balancing** | Round-robin, least-connections, IP-hash |
| **Sticky** | Cookie-based session stickiness (`_syncara_session`) |
| **WebSocket** | Detect `Upgrade: websocket`, hijack, tunnel raw TCP |
| **Metrics** | `syncara_requests_total`, `syncara_upstream_health`, `syncara_latency_seconds` |
| **Logging** | Structured JSON stderr, configurable level |
| **Lifecycle** | SIGHUP → hot reload config, SIGTERM/SIGINT → graceful drain |
| **CLI** | `syncara --config /etc/syncara.yaml` |
| **TLS** | TLS termination on listener (cert + key in config) |

What the MVP deliberately **does not** include: metrics dashboard, REST API, multi-process, Windows support, dynamic upstream registration, circuit breakers, retry logic, access logging to file.

---

## 7. Future Roadmap Boundaries

This section marks what could come next, but only after the MVP is proven in production.

- **v1.1** — Circuit breakers, retry with backoff, access log to file
- **v1.2** — HTTP/3 (QUIC) listener, gRPC passthrough (detect `application/grpc`, maintain long-lived streams)
- **v1.3** — REST API for dynamic upstream management (add/remove upstreams without restart)
- **v2.0** — Multi-node control plane (RAFT-based config sync), active health from peers
- **v2.1** — Plugin system (WebAssembly or Go plugin) for custom routing logic
- **v3.0** — Distributed edge deployment (Geo-aware routing, anycast assistance)

Each version must justify itself against the core principles. If a feature adds complexity without proportionate operational value, it stays on the cutting room floor.
