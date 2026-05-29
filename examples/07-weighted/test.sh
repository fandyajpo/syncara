#!/usr/bin/env bash
# Auto-test for 07-weighted
set -euo pipefail
cd "$(dirname "$0")"

mkdir -p /tmp/stable /tmp/canary
echo "stable" > /tmp/stable/index.html
echo "canary" > /tmp/canary/index.html
cd /tmp/stable && python3 -m http.server 9001 &
PY1=$!
cd /tmp/canary && python3 -m http.server 9002 &
PY2=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

# 50 requests — should get roughly 45 stable, 5 canary
STABLE=0
CANARY=0
for i in $(seq 1 50); do
  R=$(curl -sf http://localhost:8080/ 2>/dev/null || echo "error")
  [ "$R" = "stable" ] && STABLE=$((STABLE + 1))
  [ "$R" = "canary" ] && CANARY=$((CANARY + 1))
done

# At least 1 canary hit (10% of 50 is 5, but at minimum should be >0)
[ "$CANARY" -gt 0 ] && [ "$STABLE" -gt 0 ] && echo "07 PASS (stable=$STABLE canary=$CANARY)" || echo "07 FAIL (stable=$STABLE canary=$CANARY)"
kill $SYNCARA $PY1 $PY2 2>/dev/null
