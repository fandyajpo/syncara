#!/usr/bin/env bash
# Validation-only test for 10-production (it's a reference config)
set -euo pipefail
cd "$(dirname "$0")"

cargo run --release -p syncara -- validate -c syncara.yml 2>&1 | grep -q "valid" && echo "10 PASS" || echo "10 FAIL"
