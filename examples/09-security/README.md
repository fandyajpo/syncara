# 09 — Security Layer

**Goal:** Rate limiting, connection caps, and request validation.

```yaml
listeners:
  - port: 8080

pools:
  - name: web
    upstreams:
      - addr: "localhost:9001"
    health:
      path: /
      interval: "30s"

routes:
  - path: /
    pool: web

security:
  rate_limit:
    enabled: true
    max_requests_per_minute: 5
  connection_limits:
    max_active_connections: 100
  request_timeout: "30s"
  upstream_timeout: "10s"

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start backend
python3 -m http.server 9001 &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/09-security/syncara.yml

# Terminal 3 — trigger rate limit
for i in $(seq 1 10); do
  echo -n "Request $i: "
  curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/
done
```

Requests 1-5 return 200. Requests 6+ return 429 (rate limited).

## What you learned

- Rate limiting is per-IP using a sliding window
- Connection limits prevent upstream overload
- Request validation rejects oversized headers/URIs
- Security is opt-in — set `enabled: true` to activate
