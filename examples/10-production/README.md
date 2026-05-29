# 10 — Production

**Goal:** Production-grade config with everything enabled.

This combines all previous examples into a single production configuration:

- **Multiple listeners** — HTTP on 8080, admin/metrics on 9090
- **Host + path routing** — microservice-style routing
- **Health checks** — active probing every 10s
- **Brain scoring** — latency-aware routing
- **Sticky sessions** — for WebSocket apps
- **Rate limiting** — 300 req/min per IP
- **Connection limits** — protect upstreams
- **Timeouts** — request, upstream, WebSocket
- **Structured logging** — JSON for log aggregators

```yaml
listeners:
  - port: 8080
  - port: 8443

pools:
  - name: api
    strategy: brain
    upstreams:
      - addr: "api-1.internal:9000"
      - addr: "api-2.internal:9000"
    health:
      path: /health
      interval: "10s"
      timeout: "3s"
      unhealthy_threshold: 3
    brain:
      latency_aware: true
      health_aware: true

  - name: websocket
    strategy: sticky
    upstreams:
      - addr: "ws-1.internal:9001"
      - addr: "ws-2.internal:9001"
    session:
      cookie_name: "_syncara"
      ttl: "1h"

routes:
  - host: api.example.com
    path: /
    pool: api
  - host: ws.example.com
    path: /
    pool: websocket

security:
  rate_limit:
    enabled: true
    max_requests_per_minute: 300
  connection_limits:
    max_active_connections: 10000
    max_websocket_connections: 2000
  request_timeout: "30s"
  upstream_timeout: "30s"
  ws_timeout: "30m"

admin:
  host: "127.0.0.1"
  port: 9090

logging:
  level: info
  format: json
```

## What you learned

This is a real-world config you can adapt for your own production deployment.

## Running in production

```sh
# Validate config before deploying
syncara validate -c /etc/syncara/syncara.yml

# Run as a daemon
syncara start -c /etc/syncara/syncara.yml

# Monitor
curl http://127.0.0.1:9090/health
curl http://127.0.0.1:9090/metrics

# Hot reload config
syncara reload -c /etc/syncara/syncara.yml
```
