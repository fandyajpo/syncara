# 02 — Path Routing

**Goal:** Different URL paths go to different backends.

```yaml
listeners:
  - port: 8080

pools:
  - name: api
    upstreams:
      - addr: "localhost:9001"
  - name: static
    upstreams:
      - addr: "localhost:9002"

routes:
  - path: /api
    pool: api
  - path: /
    pool: static

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start backends
mkdir -p /tmp/api-backend /tmp/static-backend
echo '{"status":"ok"}' > /tmp/api-backend/index.html
echo '<h1>Static Site</h1>' > /tmp/static-backend/index.html

cd /tmp/api-backend && python3 -m http.server 9001 &
cd /tmp/static-backend && python3 -m http.server 9002 &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/02-path-routing/syncara.yml

# Terminal 3 — test
curl http://localhost:8080/api/
curl http://localhost:8080/
```

`/api/` hits backend 9001, everything else hits backend 9002.

## What you learned

- Routes are matched by path prefix
- First matching route wins (order matters)
- Different pools can have different upstreams
