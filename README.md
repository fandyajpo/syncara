# Syncara

**Deterministic reverse proxy with an explainable traffic brain.**

Syncara is a single-binary reverse proxy and load balancer that explains every routing decision in real time. Each request is scored across all upstreams with visible deductions (connection load, health, latency, session affinity), and the best-scored upstream is selected.

```
client ──► Syncara ──► upstream A (score: 100)
                         upstream B (score:  70 — connection load -20, health -10)
```

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/fandyajpo/syncara/main/scripts/install.sh | sh
```

Or build from source:

```sh
git clone https://github.com/fandyajpo/syncara.git
cd syncara
cargo build --release
./target/release/syncara start -c config/syncara.yml
```

## Quickstart

```sh
# Generate a starter config
syncara init

# Start the proxy
syncara start -c syncara.yml
```

Then open `http://localhost:8090` — traffic is proxied to the upstreams defined in your config.

## Features

- **Traffic Brain** — explainable scoring for every routing decision, streamed live via SSE to a dashboard
- **Multiple strategies** — least-connections, round-robin, IP-hash, random, weighted random
- **Health checks** — passive (failure counting) and active (periodic TCP/HTTP probes)
- **Sticky sessions** — cookie-based or IP-hash affinity
- **WebSocket passthrough** — full duplex upgrades, no termination
- **TLS termination** — rustls, no OpenSSL dependency
- **Config hot-reload** — SIGHUP applies config changes without downtime
- **Prometheus metrics** — `/metrics` endpoint for observability
- **Admin server** — `/health`, `/status`, `/metrics`, SSE event stream at `/events`
- **Live dashboard** — Next.js UI with real-time routing visualizations, score bars, sparklines, heat timeline
- **Security** — rate limiting, IP blocklisting, connection limits, request validation, body size limits, configurable timeouts

## Architecture

A single Rust binary with zero runtime dependencies. The proxy core handles HTTP/1.1, HTTP/2, and WebSocket upgrades. The Traffic Brain scores every upstream before each routing decision and broadcasts every decision to dashboard clients via SSE.

```
Config ──► Router ──► Balancer ──► Proxy ──► Upstream Pool
                          │
                          ▼
                    Traffic Brain ──► SSE ──► Dashboard
```

## License

MIT
