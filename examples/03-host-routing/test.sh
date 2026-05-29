#!/usr/bin/env bash
# Auto-test for 03-host-routing
set -euo pipefail
cd "$(dirname "$0")"

python3 -m http.server 9001 &
PY1=$!
python3 -m http.server 9002 &
PY2=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

A=$(curl -sf -H "Host: app-a.example.com" -o /dev/null -w "%{http_code}" http://localhost:8080/)
B=$(curl -sf -H "Host: app-b.example.com" -o /dev/null -w "%{http_code}" http://localhost:8080/)
[ "$A" = "200" ] && echo -n "APP-A OK " || echo -n "APP-A FAIL "
[ "$B" = "200" ] && echo -n "APP-B OK " || echo -n "APP-B FAIL "
echo "03 PASS"
kill $SYNCARA $PY1 $PY2 2>/dev/null
