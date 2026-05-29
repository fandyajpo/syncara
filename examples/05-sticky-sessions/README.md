# 05 — Sticky Sessions

**Goal:** A client always hits the same backend (cookie-based stickiness).

```yaml
listeners:
  - port: 8080

pools:
  - name: web
    strategy: sticky
    upstreams:
      - addr: "localhost:9001"
      - addr: "localhost:9002"
    session:
      cookie_name: "_syncara"
      ttl: "5m"

routes:
  - path: /
    pool: web

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start backends with visible IDs
mkdir -p /tmp/b1 /tmp/b2
echo "BACKEND 9001" > /tmp/b1/index.html
echo "BACKEND 9002" > /tmp/b2/index.html
cd /tmp/b1 && python3 -m http.server 9001 &
cd /tmp/b2 && python3 -m http.server 9002 &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/05-sticky-sessions/syncara.yml

# Terminal 3 — same client, always same backend
curl -c /tmp/cookies.txt -b /tmp/cookies.txt http://localhost:8080/
curl -c /tmp/cookies.txt -b /tmp/cookies.txt http://localhost:8080/
curl -c /tmp/cookies.txt -b /tmp/cookies.txt http://localhost:8080/
```

All three requests hit the same backend. Without cookies, round-robin would vary them.

## What you learned

- `strategy: sticky` uses cookies to pin clients to backends
- The `_syncara` cookie tells Syncara which backend to use
- Great for WebSocket apps, shopping carts, dashboards
