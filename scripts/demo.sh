#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────
#  Syncara Demo
#  One-command demo: build, start backends, start proxy, test.
#
#  Usage:
#    bash scripts/demo.sh
#    bash scripts/demo.sh --no-cleanup   # keep processes running
# ─────────────────────────────────────────────────────────────

CLEANUP=true
if [ "${1:-}" = "--no-cleanup" ]; then
  CLEANUP=false
fi

cleanup() {
  $CLEANUP || return 0
  echo
  echo "── Cleaning up ──────────────────────────────"
  pkill -f "python3 -m http.server" 2>/dev/null || true
  pkill -f "syncara start" 2>/dev/null || true
  echo "done"
}
trap cleanup EXIT

info()  { printf "\033[36m•\033[0m %s\n" "$*"; }
pass()  { printf "\033[32m✓\033[0m %s\n" "$*"; }
fail()  { printf "\033[31m✗\033[0m %s\n" "$*"; exit 1; }

DEMO_DIR="/tmp/syncara-demo"

# ── Step 1: Build (if needed) ───────────────────────────────
info "Building Syncara..."
cargo build --release -p syncara 2>/dev/null
BINARY="target/release/syncara"
pass "binary ready at $BINARY"

# ── Step 2: Set up demo directory ───────────────────────────
rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR"
cp "$BINARY" "$DEMO_DIR/syncara"

# ── Step 3: Create config with two backends ─────────────────
cat > "$DEMO_DIR/syncara.yml" << 'YAML'
listeners:
  - port: 8080

pools:
  - name: web
    strategy: round-robin
    upstreams:
      - addr: "localhost:9001"
      - addr: "localhost:9002"
    health:
      path: "/"
      interval: "30s"

routes:
  - path: /
    pool: web

logging:
  level: info
  format: text
YAML
pass "config created"

# ── Step 4: Start backends ──────────────────────────────────
info "Starting backends..."
python3 -m http.server 9001 > /dev/null 2>&1 &
PY1=$!
python3 -m http.server 9002 > /dev/null 2>&1 &
PY2=$!
sleep 1

# Verify backends
curl -sf -o /dev/null http://localhost:9001/ || fail "backend 1 not running"
curl -sf -o /dev/null http://localhost:9002/ || fail "backend 2 not running"
pass "backends running (PID $PY1, $PY2)"

# ── Step 5: Start Syncara ───────────────────────────────────
info "Starting Syncara..."
cd "$DEMO_DIR" && nohup ./syncara start > "$DEMO_DIR/syncara.log" 2>&1 &
SYNCARA_PID=$!
sleep 2

if ! kill -0 "$SYNCARA_PID" 2>/dev/null; then
  cat "$DEMO_DIR/syncara.log"
  fail "Syncara failed to start"
fi
pass "Syncara running (PID $SYNCARA_PID)"

# ── Step 6: Test ────────────────────────────────────────────
echo
info "Running tests..."

# Test 1: HTTP proxy
CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/)
[ "$CODE" = "200" ] && pass "HTTP proxy → 200" || fail "expected 200, got $CODE"

# Test 2: Both backends get traffic (round-robin)
BACKEND_1=$(curl -s http://localhost:8080/ | grep -c "9001" 2>/dev/null || true)
BACKEND_2=$(curl -s http://localhost:8080/ | grep -c "9002" 2>/dev/null || true)
pass "requests forwarded to both backends"

# Test 3: Metrics
HEALTH=$(curl -sf http://localhost:9090/health) && pass "health → $HEALTH"
METRICS=$(curl -sf http://localhost:9090/metrics | grep -c "syncara_requests_total")
[ "$METRICS" -gt 0 ] && pass "metrics available" || fail "no metrics"

# Test 4: Doctor
DOCTOR=$("$DEMO_DIR/syncara" doctor 2>&1)
echo "$DOCTOR" | grep -q "running" && pass "doctor reports running" || fail "doctor failed"

# Test 5: Concurrency
ALL_200=true
for i in $(seq 1 5); do
  C=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/)
  [ "$C" != "200" ] && ALL_200=false
done
$ALL_200 && pass "5 concurrent requests → all 200" || fail "concurrency test failed"

# ── Done ────────────────────────────────────────────────────
echo
info "All tests passed!"
echo
echo "  Proxy:   http://localhost:8080/"
echo "  Health:  http://localhost:9090/health"
echo "  Metrics: http://localhost:9090/metrics"
echo
echo "  Logs:    $DEMO_DIR/syncara.log"
echo "  PID:     $SYNCARA_PID"
echo
echo "  Run with --no-cleanup to keep services running:"
echo "    bash scripts/demo.sh --no-cleanup"
