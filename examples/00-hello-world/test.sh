#!/usr/bin/env bash
# Auto-test for 00-hello-world
set -euo pipefail
cd "$(dirname "$0")"

python3 -m http.server 9001 &
BACKEND=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

curl -sf -o /dev/null -w "%{http_code}" http://localhost:8080/ && echo " 00 PASS" || echo " 00 FAIL"
kill $SYNCARA $BACKEND 2>/dev/null
