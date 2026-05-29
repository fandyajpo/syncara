# 06 — WebSocket Proxy

**Goal:** Syncara proxies WebSocket connections to a backend.

```yaml
listeners:
  - port: 8080

pools:
  - name: ws
    upstreams:
      - addr: "localhost:9003"

routes:
  - path: /
    pool: ws

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start a WebSocket echo server
node examples/websocket-chat/server.js

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/06-websocket/syncara.yml

# Terminal 3 — connect through Syncara
cargo install websocat
websocat ws://localhost:8080/
# type a message, it echoes back
```

Syncara detects the `Upgrade: websocket` header and tunnels the connection.

## Without Node.js (using python3 + ws proto)

If you don't have Node.js, just test the upgrade detection:

```sh
curl -H "Upgrade: websocket" -H "Connection: Upgrade" http://localhost:8080/
# → 502 because Python isn't a WS server,
#   but Syncara correctly detected the WS upgrade attempt
```

## What you learned

- WebSocket works out of the box — no special config needed
- Syncara detects WS upgrade by HTTP headers
- The connection is tunneled bidirectionally
- WebSocket timeouts are configurable in the security settings
