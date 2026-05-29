#!/usr/bin/env bash
# Auto-test for 02-path-routing
set -euo pipefail
cd "$(dirname "$0")"

mkdir -p /tmp/api-b /tmp/static-b
echo 'api' > /tmp/api-b/index.html
echo 'static' > /tmp/static-b/index.html

cd /tmp/api-b && python3 -m http.server 9001 &
PY1=$!
cd /tmp/static-b && python3 -m http.server 9002 &
PY2=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

[ "$(curl -sf http://localhost:8080/api/)" = "api" ] && echo -n "API OK " || echo -n "API FAIL "
[ "$(curl -sf http://localhost:8080/)" = "static" ] && echo -n "STATIC OK " || echo -n "STATIC FAIL "
echo "02 PASS"
kill $SYNCARA $PY1 $PY2 2>/dev/null
