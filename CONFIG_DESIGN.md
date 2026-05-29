# Syncara Configuration Design

## Design Note: YAML over TOML

The architecture document (ARCHITECTURE.md) initially specified TOML. After further consideration, **Syncara v1 uses YAML**.

Rationale:

| Concern | TOML | YAML |
|---|---|---|
| Ecosystem familiarity | Rust-native, favored by Rust projects | Ubiquitous in infra tooling (Docker, K8s, Ansible, Prometheus, Caddy) |
| Nested config | Tables `[a.b.c]` work but are verbose | Natural tree structure with indentation |
| Comments | `#` line comments only | `#` line comments, clearer for documentation |
| Config standards | Few infra tools use TOML | YAML is the de-facto standard for infra config |
| Readability | Good for flat configs | Better for hierarchical configs (listeners → routes → pools) |
| Deserialization | `serde` support is excellent | `serde` + `serde_yaml` support is equally mature |

**Decision**: YAML for config file. Internal config representation remains `serde`-derived regardless of format, so switching to TOML or JSON later is trivial.

---

## Configuration Philosophy

1. **Config is the product API.** Every feature a user interacts with begins in the config file. The config must be discoverable, self-documenting, and predictable.

2. **Progressive complexity.** A single upstream behind a single listener needs 7 lines. Adding health checks, TLS, and sticky sessions adds optional blocks — it never removes or restructures what was already there.

3. **Explicit over implicit.** No magic defaults that hide behavior. If a feature is enabled, the config says so. Defaults are only for values that have an obvious safe choice (e.g., `interval: 10s` for health checks).

4. **Fail at parse time, not at runtime.** Invalid references (route → nonexistent pool, pool → no upstreams) produce clear error messages at startup. The binary refuses to start with a broken config.

5. **One file, one source of truth.** No includes, no overlays, no environment variable interpolation in v1. If a deployment needs multiple configs, run multiple Syncara processes.

---

## 1. YAML Schema

### 1.1 Top-Level Structure

```yaml
# Listener ports where Syncara accepts connections
listeners:
  - port: <uint16>
    host: <string>              # optional, default "0.0.0.0"
    tls:                        # optional
      cert: <string>            # path to PEM certificate
      key: <string>             # path to PEM private key

# Route rules: which requests go to which pool
routes:
  - host: <string>              # optional, match Host header
    path: <string>              # optional, match path prefix
    pool: <string>              # required, must match a pool name
    websocket: <bool>           # optional, default false

# Upstream server pools
pools:
  - name: <string>              # required, referenced by routes
    strategy: <string>          # optional: "round-robin" (default),
                                #   "least-connections", "ip-hash"
    upstreams:
      - addr: <string>          # required: "host:port"
        weight: <uint>          # optional, default 1
    health:                     # optional, whole block
      path: <string>            # optional, default "/health"
      interval: <duration>      # optional, default "10s"
      timeout: <duration>       # optional, default "3s"
      unhealthy_threshold: <uint>  # optional, default 2
      healthy_threshold: <uint>    # optional, default 1
    session:                    # optional, whole block
      cookie: <string|bool>     # optional, cookie name or true
      ttl: <duration>           # optional, default "24h"

# Observability settings
logging:
  level: <string>               # optional: "trace", "debug", "info",
                                #   "warn", "error" (default "info")
  format: <string>              # optional: "json" (default), "text"

# Admin / metrics server
admin:
  port: <uint16>                # optional, default 9090
  host: <string>                # optional, default "127.0.0.1"
```

### 1.2 Duration Format

All duration values use Go-style duration strings (or human-readable in the Rust implementation):

- `10s`, `30s`, `5m`, `1h`, `24h`
- Minimum resolution: 1 second
- Must be positive and non-zero

---

## 2. Validation Strategy

### 2.1 Parse-Time Validation (hard failures — refuse to start)

| Rule | Error Message |
|---|---|
| At least one listener | `no listeners defined (need at least 1)` |
| No duplicate listener `host:port` | `listener[0] and listener[2] both bind to "0.0.0.0:8080"` |
| At least one route | `no routes defined (need at least 1)` |
| Route pool references an existing pool | `route[1] references pool "api" which is not defined` |
| At least one upstream per pool | `pool "api" has no upstreams` |
| No duplicate upstream addresses in a pool | `pool "api" contains duplicate upstream "10.0.0.1:3000"` |
| TLS cert and key both present | `listener[0] has tls.cert but no tls.key` |
| Cert file exists | `tls.cert "/etc/certs/foo.pem": file not found` |
| Key file exists | `tls.key "/etc/certs/foo.key": file not found` |
| Health check interval > timeout | `pool "api": health.interval (5s) must be greater than health.timeout (10s)` |
| Healthy threshold >= 1 | `pool "api": health.healthy_threshold must be >= 1` |
| Unhealthy threshold >= 1 | `pool "api": health.unhealthy_threshold must be >= 1` |
| Port in valid range (1–65535) | `listener[0]: port 0 is out of range` |
| `"ip-hash"` strategy requires `session` block | *warning only (see below)* |
| Log level is recognized | `logging.level "critical" is not valid (must be one of: trace, debug, info, warn, error)` |

### 2.2 Warning-Time Validation (soft — log warning, start anyway)

| Warning | Rationale |
|---|---|
| `pool "api" has no health checks configured` | User may have forgotten to enable health checks |
| `pool "api" uses "ip-hash" strategy without session block` | ip-hash without session might not be intentional |
| `route[0] matches host "*.example.com" with no pool health checks` | Wildcard routes without health checks can route to dead upstreams |
| `listener[0] TLS configured but route[1] has websocket: true` | Informational; user should know WebSocket over TLS is expected |

### 2.3 Validation Architecture

```
Config file (YAML)
  │
  ▼
serde_yaml::from_str()        — structural validation (types, required fields)
  │
  │  failure → error with line number and YAML path
  ▼
ConfigValidator::validate()   — semantic validation (cross-field rules)
  │
  │  failure → error messages, one per rule violation
  ▼
Validated Config             — guaranteed internally consistent
```

**Cross-field validation** uses a dedicated `validate()` function (not scattered across modules). This function iterates all rules and collects errors into a `Vec<ConfigError>`. If any errors are found, none are logged — the full list is printed and the process exits with code 1.

```rust
// Pseudocode for the validation approach
fn validate(cfg: &Config) -> Result<(), Vec<ConfigError>> {
    let mut errors = vec![];

    if cfg.listeners.is_empty() {
        errors.push(err("no listeners defined"));
    }

    for (i, route) in cfg.routes.iter().enumerate() {
        if !cfg.pools.iter().any(|p| p.name == route.pool) {
            errors.push(err(format!("route[{i}] references unknown pool '{}'", route.pool)));
        }
    }

    for pool in &cfg.pools {
        if pool.upstreams.is_empty() {
            errors.push(err(format!("pool '{}' has no upstreams", pool.name)));
        }
    }

    // ... all other rules ...

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

---

## 3. Config Loading Philosophy

### Loading order

```
1. CLI: syncara --config /etc/syncara/syncara.yml [--validate-only]
2. Default search path (in order, first found wins):
   a. ./syncara.yml
   b. ./syncara.yaml
   c. /etc/syncara/syncara.yml
   d. /etc/syncara/syncara.yaml
   e. $XDG_CONFIG_HOME/syncara/syncara.yml
3. --validate-only flag: parse + validate, then exit 0 or 1 (no server start)
```

### Hot reload (SIGHUP)

- Config is re-read and re-validated on SIGHUP
- If validation fails: log errors, keep running on old config
- If validation succeeds: atomically swap the active config
- In-flight connections complete against the old config
- New connections use the new config

### Environment variables

Syncara v1 does **not** support environment variable interpolation in config files. Environment variables for sensitive values (e.g., TLS paths) is a v1.1 feature. For v1, TLS cert/key paths are read from the config file directly.

---

## 4. Defaults Reference

| Setting | Default | Why |
|---|---|---|
| `listener[].host` | `"0.0.0.0"` | Standard default, accept on all interfaces |
| `pool[].strategy` | `"round-robin"` | Simplest, most predictable |
| `pool[].upstream[].weight` | `1` | Equal distribution by default |
| `pool[].health.path` | `"/health"` | Common convention |
| `pool[].health.interval` | `"10s"` | Frequent enough for fast failure detection, sparse enough to avoid load |
| `pool[].health.timeout` | `"3s"` | Long enough for a TCP connect + HTTP round trip, short enough to not block the checker loop |
| `pool[].health.unhealthy_threshold` | `2` | One failure could be transient; two confirms the issue |
| `pool[].health.healthy_threshold` | `1` | One successful check is enough to restore (conservative for recovery) |
| `pool[].session.cookie` | `"_syncara_session"` | Cookie name if session is enabled |
| `pool[].session.ttl` | `"24h"` | Long enough for most user sessions |
| `pool[].websocket` | `false` | Opt-in — not all routes need WebSocket support |
| `logging.level` | `"info"` | Informational messages + errors, no debug noise |
| `logging.format` | `"json"` | Structured output for log aggregation |
| `admin.port` | `9090` | Standard non-privileged metrics port |
| `admin.host` | `"127.0.0.1"` | Admin endpoint is not exposed to the network by default |

### What has NO default (must be explicit)

| Field | Why |
|---|---|
| `listener[].port` | Which port to bind is a security-sensitive decision |
| `pool[].name` | Must be explicit for route references |
| `pool[].upstreams` | Must be explicit — zero upstreams is useless |
| `route[].pool` | Must be explicit — no magic routing |
| `tls.cert` / `tls.key` | Paths depend on deployment environment |

---

## 5. Beginner Examples

### 5.1 Single upstream, no frills

```yaml
# syncara.yml — 7 lines, production-ready minimal
listeners:
  - port: 8080

routes:
  - pool: default

pools:
  - upstreams:
      - addr: "127.0.0.1:3000"
```

**What this does**: Listens on port 8080, forwards every request to `127.0.0.1:3000`. Defaults to round-robin (irrelevant with one upstream).

### 5.2 Two upstreams, round-robin

```yaml
listeners:
  - port: 8080

routes:
  - pool: web

pools:
  - name: web
    upstreams:
      - addr: "127.0.0.1:3000"
      - addr: "127.0.0.1:3001"
```

### 5.3 TLS termination

```yaml
listeners:
  - port: 443
    tls:
      cert: /etc/letsencrypt/live/example.com/fullchain.pem
      key: /etc/letsencrypt/live/example.com/privkey.pem

routes:
  - pool: web

pools:
  - name: web
    upstreams:
      - addr: "127.0.0.1:3000"
      - addr: "127.0.0.1:3001"
```

---

## 6. Advanced Examples

### 6.1 Multi-service with health checks and sticky sessions

```yaml
listeners:
  - port: 80
  - port: 443
    tls:
      cert: /etc/certs/example.com.pem
      key: /etc/certs/example.com-key.pem

routes:
  - host: app.example.com
    pool: web-backend
  - host: api.example.com
    path: /v1
    pool: api-v1
    websocket: true
  - host: api.example.com
    path: /v2
    pool: api-v2
  - host: static.example.com
    pool: static-servers

pools:
  - name: web-backend
    strategy: least-connections
    upstreams:
      - addr: "10.0.1.10:8080"
      - addr: "10.0.1.11:8080"
      - addr: "10.0.1.12:8080"
    health:
      path: /healthz
      interval: 5s
      timeout: 2s
      unhealthy_threshold: 3
    session:
      cookie: _sync_session
      ttl: 1h

  - name: api-v1
    strategy: round-robin
    upstreams:
      - addr: "10.0.2.10:9000"
      - addr: "10.0.2.11:9000"
    health:
      path: /health
    session:
      cookie: true
    websocket: true

  - name: api-v2
    strategy: least-connections
    upstreams:
      - addr: "10.0.3.10:9090"
      - addr: "10.0.3.11:9090"
    health:
      path: /ready

  - name: static-servers
    strategy: ip-hash
    upstreams:
      - addr: "10.0.4.10:80"
      - addr: "10.0.4.11:80"
```

### 6.2 Wildcard host, weighted upstreams, passive-only health

```yaml
listeners:
  - port: 8080

routes:
  - host: "*.example.com"
    pool: backend

pools:
  - name: backend
    strategy: least-connections
    upstreams:
      - addr: "10.0.0.10:3000"
        weight: 5
      - addr: "10.0.0.11:3000"
        weight: 3
      - addr: "10.0.0.12:3000"
        weight: 2
    health:
      path: /health
    session:
      cookie: true
```

**Weight behavior**: out of 10 total weight units, upstream `10.0.0.10` receives ~50% of traffic, `10.0.0.11` receives ~30%, `10.0.0.12` receives ~20%.

### 6.3 Path-based routing with a catch-all

```yaml
listeners:
  - port: 80

routes:
  - path: /api/v1
    pool: v1-api
  - path: /api/v2
    pool: v2-api
  - path: /
    pool: frontend

pools:
  - name: v1-api
    upstreams:
      - addr: "10.0.0.10:4000"
  - name: v2-api
    upstreams:
      - addr: "10.0.0.10:4001"
  - name: frontend
    upstreams:
      - addr: "10.0.0.20:3000"
```

**Matching behavior**: The first matching route wins. Route order in the config file defines priority. `/` is the catch-all and must be last.

---

## 7. Route Matching Rules

| `host` | `path` | Match behavior |
|---|---|---|
| — | — | Matches every request (catch-all) |
| `"example.com"` | — | Exact Host header match |
| `"*.example.com"` | — | Wildcard: matches `foo.example.com`, `bar.example.com`, but not `example.com` or `test.foo.example.com` |
| — | `"/api"` | Path **prefix** match: matches `/api`, `/api/v1`, `/api/v1/foo`, but not `/apiary` |
| — | `"/api/"` | Path prefix match with trailing slash: matches `/api/v1`, but not `/api` |
| — | `"/api/*"` | Not supported in v1 (prefix matching covers all cases) |
| `"example.com"` | `"/api"` | Both must match: Host header AND path prefix |

**No regex in v1.** Wildcard host + prefix path covers the vast majority of real-world routing needs. Regex routes can be added in a future release behind a feature flag without breaking existing configs.

---

## 8. Future Extensibility Without Breaking Changes

| Future Feature | Config Impact | Backward Compat? |
|---|---|---|
| Rate limiting | `route[].rate_limit: { max: 100, window: 1s }` | Adding optional field — no breakage |
| Request rewriting | `route[].rewrite: { host: "...", path: "..." }` | Adding optional block — no breakage |
| Response headers | `route[].response_headers: { "X-Frame-Options": "DENY" }` | Adding optional field — no breakage |
| Access log to file | `logging.access_log: "/var/log/syncara/access.log"` | Adding optional field — no breakage |
| Circuit breaker | `pool[].circuit_breaker: { max_fails: 5, half_open_after: 30s }` | Adding optional block — no breakage |
| Upstream TLS | `pool[].upstream[].tls: { cert: "...", key: "..." }` | Adding optional field — no breakage |
| Environment interpolation | `${VAR}` in config values | Adding processing step — no breakage |
| Config includes | `includes: ["base.yml", "routes/*.yml"]` | Adding top-level field — no breakage |
| Multiple config files | `syncara --config-dir /etc/syncara/conf.d/` | Adding CLI flag — no breakage |

**Guarantee**: Any config file that works in v1 will work unchanged in v1.x and v2.0. New features are added as optional fields and optional blocks. Existing fields never change semantics.

---

## 9. Config File Format Detection

```yaml
# Determined by file extension:
#   .yml  or  .yaml  → YAML
#   .toml            → TOML (future)
#   .json            → JSON (future)
#
# No extension → try YAML first, fall back to TOML, then JSON.
# Explicit format via --config-format flag (future).
```

For v1, only `.yml` / `.yaml` is supported.

---

## 10. Summary: Config Design Principles

| Principle | Practice |
|---|---|
| Progressive complexity | Minimal config is 7 lines. Add optional blocks as needed. |
| Explicit over implicit | No magic auto-discovery. Everything is declared. |
| Fail early | Invalid config = refuse to start. Every error has a clear message. |
| One file | No includes, no overlays, no env var injection in v1. |
| Backward compatible forever | New fields are always optional. Existing fields never change. |
| Predictable routing | First matching route wins. Order in file defines priority. |
| Sensible defaults | Every optional field has a safe, documented default. |

---

## Appendix: Minimal Config Reference Card

```yaml
# Minimal (7 lines)
listeners:
  - port: 8080
routes:
  - pool: default
pools:
  - upstreams:
      - addr: "127.0.0.1:3000"

# With health checks
pools:
  - name: default
    health:
      path: /health
    upstreams:
      - addr: "127.0.0.1:3000"

# With TLS
listeners:
  - port: 443
    tls:
      cert: /etc/certs/cert.pem
      key: /etc/certs/key.pem

# With sticky sessions
pools:
  - name: default
    session:
      cookie: true
    upstreams:
      - addr: "127.0.0.1:3000"

# With WebSocket
routes:
  - host: ws.example.com
    pool: ws-backend
    websocket: true
```
