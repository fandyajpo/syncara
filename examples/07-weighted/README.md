# 07 — Weighted (Canary Deploy)

**Goal:** Send 10% of traffic to a new version (canary).

```yaml
listeners:
  - port: 8080

pools:
  - name: web
    strategy: weighted
    upstreams:
      - addr: "localhost:9001"
        weight: 90
      - addr: "localhost:9002"
        weight: 10

routes:
  - path: /
    pool: web

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start backends with visible labels
mkdir -p /tmp/stable /tmp/canary
echo "STABLE v1" > /tmp/stable/index.html
echo "CANARY v2" > /tmp/canary/index.html
cd /tmp/stable && python3 -m http.server 9001 &
cd /tmp/canary && python3 -m http.server 9002 &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/07-weighted/syncara.yml

# Terminal 3 — hit it 20 times, count canary hits
for i in $(seq 1 20); do curl -s http://localhost:8080/; done
```

About 2 out of 20 go to the canary. Adjust `weight` for finer control.

## What you learned

- `weighted` strategy distributes traffic proportionally
- Weight 90 = 90% of requests (9:1 ratio)
- Perfect for canary deploys, A/B testing, gradual rollouts
