#!/usr/bin/env bash
# Auto-test for 06-websocket
set -euo pipefail
cd "$(dirname "$0")"

# Use python3 to simulate a raw WS server (not a real handshake, just check upgrade)
python3 -c "
import socket, threading
def handle(c):
    c.recv(1024)
    c.send(b'HTTP/1.1 101 Switching Protocols\r\n\r\n')
    c.send(b'hello from ws backend')
    c.close()
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('', 9003))
s.listen(1)
while True:
    threading.Thread(target=handle, args=(s.accept()[0],), daemon=True).start()
" &
BACKEND=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

# WS upgrade header should get 101
CODE=$(curl -sf -o /dev/null -w "%{http_code}" \
  -H "Upgrade: websocket" \
  -H "Connection: Upgrade" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  http://localhost:8080/ 2>/dev/null || echo "fail")

[ "$CODE" = "101" ] && echo "06 PASS" || echo "06 FAIL (got $CODE)"
kill $SYNCARA $BACKEND 2>/dev/null
