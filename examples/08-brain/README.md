# 08 — Brain Scoring

**Goal:** Syncara's Brain routes traffic to the healthiest, least-loaded, lowest-latency backend.

```yaml
listeners:
  - port: 8080

pools:
  - name: web
    strategy: brain
    upstreams:
      - addr: "localhost:9001"
      - addr: "localhost:9002"
    brain:
      latency_aware: true
      health_aware: true

routes:
  - path: /
    pool: web

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start backends (one slightly slower)
python3 -m http.server 9001 &
python3 -c "
import http.server, socketserver, time
class SlowHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        time.sleep(0.05)  # 50ms delay
        super().do_GET()
socketserver.TCPServer.allow_reuse_address = True
s = socketserver.TCPServer(('', 9002), SlowHandler)
s.serve_forever()
" &

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/08-brain/syncara.yml

# Terminal 3 — hit it, see Brain prefer the faster backend
for i in $(seq 1 10); do curl -s -o /dev/null -w "%{time_total}s\n" http://localhost:8080/; done
```

The Brain learns that 9002 is 50ms slower and sends most traffic to 9001.

## What you learned

- Brain scoring considers: health, active connections, latency
- Lower latency → gets more traffic
- No ML, no training — deterministic weighted scoring
- Each decision is logged with a breakdown of scores
