# WebSocket Chat Backend

Tests Syncara's WebSocket proxying.

## Run

```sh
docker compose up
```

## Test with websocat

```sh
cargo install websocat
websocat ws://localhost:8080/
# → "connected to syncara backend"
# type something, it echoes back
```

## Test with curl

```sh
# Syncara detects WS upgrade headers automatically
curl -H "Upgrade: websocket" -H "Connection: Upgrade" http://localhost:8080/
# → 101 Switching Protocols (if backend supports WS)
```
