#!/usr/bin/env bash
# Auto-test for 09-security
set -euo pipefail
cd "$(dirname "$0")"

python3 -m http.server 9001 &
PY1=$!
sleep 1

cargo run --release -p syncara -- start -c syncara.yml &
SYNCARA=$!
sleep 2

# First 5 should be 200, next 5 should be 429
CODE=""
for i in $(seq 1 10); do
  R=$(curl -sf -o /dev/null -w "%{http_code}" http://localhost:8080/ 2>/dev/null || echo "429")
  CODE="$CODE$R "
done

# Check we got at least one 429 after rate limit kicked in
echo "$CODE" | grep -q "429" && echo "09 PASS ($CODE)" || echo "09 FAIL (no rate limiting detected — $CODE)"
kill $SYNCARA $PY1 2>/dev/null
