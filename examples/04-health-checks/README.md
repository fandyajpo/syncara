# 04 — Health Checks

**Goal:** Syncara detects unhealthy backends and stops sending them traffic.

```yaml
listeners:
  - port: 8080

pools:
  - name: web
    strategy: round-robin
    upstreams:
      - addr: "localhost:9001"
      - addr: "localhost:9002"
    health:
      path: /
      interval: "5s"
      timeout: "2s"
      unhealthy_threshold: 1
      healthy_threshold: 1

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
cargo run --release -p syncara -- start -c examples/04-health-checks/syncara.yml

# Terminal 3 — see balancer working, then kill one backend
for i in $(seq 1 4); do curl -s http://localhost:8080/ | head -1; sleep 1; done
kill %1  # kill backend 9001
echo "--- backend 9001 is dead ---"
for i in $(seq 1 4); do curl -s http://localhost:8080/ | head -1; sleep 1; done
```

Watch the requests stop going to the dead backend within 5 seconds.

## What you learned

- Health checks probe each upstream on a timer
- Unhealthy upstreams are removed from rotation
- They come back automatically when healthy again
