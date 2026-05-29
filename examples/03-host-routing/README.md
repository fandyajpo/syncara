# 03 — Host Routing

**Goal:** Different domain names route to different backends (virtual hosting).

```yaml
listeners:
  - port: 8080

pools:
  - name: app-a
    upstreams:
      - addr: "localhost:9001"
  - name: app-b
    upstreams:
      - addr: "localhost:9002"

routes:
  - host: app-a.example.com
    path: /
    pool: app-a
  - host: app-b.example.com
    path: /
    pool: app-b

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start backends
python3 -m http.server 9001 &
python3 -m http.server 9002 &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/03-host-routing/syncara.yml

# Terminal 3 — test with Host header
curl -H "Host: app-a.example.com" http://localhost:8080/
curl -H "Host: app-b.example.com" http://localhost:8080/
```

Same proxy, two different "sites" served from the same port.

## What you learned

- Routes can match by `host`, `path`, or both
- `Host` header determines which route matches
- Useful for multi-tenant or microservice setups
