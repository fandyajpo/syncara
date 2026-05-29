#!/usr/bin/env bash
# Auto-test for 01-two-backends
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

for i in 1 2 3; do
  curl -sf -o /dev/null -w "%{http_code} " http://localhost:8080/ || echo -n "FAIL "
done
echo "01 PASS"
kill $SYNCARA $PY1 $PY2 2>/dev/null
