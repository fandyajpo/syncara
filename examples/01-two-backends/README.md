# 01 — Two Backends

**Goal:** Round-robin load balancing between two upstreams.

```yaml
listeners:
  - port: 8080

pools:
  - name: web
    strategy: round-robin
    upstreams:
      - addr: "localhost:9001"
      - addr: "localhost:9002"

routes:
  - path: /
    pool: web

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start two backends
python3 -m http.server 9001 &
python3 -m http.server 9002 &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/01-two-backends/syncara.yml

# Terminal 3 — see requests alternate between backends
curl http://localhost:8080/
curl http://localhost:8080/
curl http://localhost:8080/
```

Each request goes to a different backend in turn.

## What you learned

- `pools` let you group multiple upstreams
- `strategy: round-robin` distributes requests evenly
- Routes reference pools by `name`
- You can use `proxy:` shorthand or explicit `pools` + `routes`
