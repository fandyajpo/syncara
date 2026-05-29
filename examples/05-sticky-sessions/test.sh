#!/usr/bin/env bash
# Auto-test for 05-sticky-sessions
set -euo pipefail
cd "$(dirname "$0")"

mkdir -p /tmp/sb1 /tmp/sb2
echo "B1" > /tmp/sb1/index.html
echo "B2" > /tmp/sb2/index.html
cd /tmp/sb1 && python3 -m http.server 9001 &
PY1=$!
cd /tmp/sb2 && python3 -m http.server 9002 &
PY2=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

# Same cookie jar → same backend every time
R1=$(curl -sf -c /tmp/sticky-cookies.txt -b /tmp/sticky-cookies.txt http://localhost:8080/)
R2=$(curl -sf -c /tmp/sticky-cookies.txt -b /tmp/sticky-cookies.txt http://localhost:8080/)
R3=$(curl -sf -c /tmp/sticky-cookies.txt -b /tmp/sticky-cookies.txt http://localhost:8080/)

if [ "$R1" = "$R2" ] && [ "$R2" = "$R3" ]; then
  echo "05 PASS (all same backend: $R1)"
else
  echo "05 FAIL (got: $R1 $R2 $R3)"
fi
rm -f /tmp/sticky-cookies.txt
kill $SYNCARA $PY1 $PY2 2>/dev/null
