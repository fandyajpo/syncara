#!/usr/bin/env bash
# Auto-test for 08-brain
set -euo pipefail
cd "$(dirname "$0")"

# Fast backend
python3 -m http.server 9001 &
PY1=$!

# Slow backend (50ms delay)
python3 -c "
import http.server, socketserver, time
class H(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        time.sleep(0.05)
        super().do_GET()
socketserver.TCPServer.allow_reuse_address = True
s = socketserver.TCPServer(('', 9002), H)
s.serve_forever()
" &
PY2=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

# Send 20 requests — most should hit the fast backend
FAST=0
SLOW=0
for i in $(seq 1 20); do
  T=$(curl -sf -o /dev/null -w "%{time_total}" http://localhost:8080/ 2>/dev/null || echo "0")
  # <40ms → fast backend, >40ms → slow backend
  python3 -c "exit(0 if $T < 0.04 else 1)" && FAST=$((FAST + 1)) || SLOW=$((SLOW + 1))
done

[ "$FAST" -gt "$SLOW" ] && echo "08 PASS (fast=$FAST slow=$SLOW)" || echo "08 FAIL (fast=$FAST slow=$SLOW — brain should prefer fast backend)"
kill $SYNCARA $PY1 $PY2 2>/dev/null
