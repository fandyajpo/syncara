# 00 — Hello World

**Goal:** Get traffic flowing through Syncara. Minimal config.

```yaml
listeners:
  - port: 8080

routes:
  - path: /
    proxy: http://localhost:9001

logging:
  level: info
  format: text
```

## Run it

```sh
# Terminal 1 — start a backend
python3 -m http.server 9001

# Terminal 2 — start syncara
cargo run --release -p syncara -- start -c examples/00-hello-world/syncara.yml

# Terminal 3 — test
curl http://localhost:8080/
```

You should see a directory listing from the Python server, served through Syncara.

## What you learned

- Syncara reads a YAML file and starts a proxy on the configured port
- `proxy:` is shorthand that creates a pool with one upstream
- Requests flow: curl → Syncara (:8080) → backend (:9001)
