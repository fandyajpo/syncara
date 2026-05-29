#!/usr/bin/env bash
# Auto-test for 04-health-checks
set -euo pipefail
cd "$(dirname "$0")"

python3 -m http.server 9001 &
PY1=$!
python3 -m http.server 9002 &
PY2=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 6  # wait for health checks to mark both healthy

# Both should work
curl -sf -o /dev/null -w "%{http_code}" http://localhost:8080/ || echo -n "INIT FAIL "

# Kill one backend
kill $PY1 2>/dev/null || true
sleep 6  # wait for health check to detect

# Should still get 200 from the remaining backend
curl -sf -o /dev/null -w "%{http_code}" http://localhost:8080/ && echo "04 PASS" || echo "04 FAIL"
kill $SYNCARA $PY2 2>/dev/null
