#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
#  Syncara — Homebrew formula generator
#
#  Substitutes version and SHA256 placeholders in the template.
#  Called by CI during the release workflow.
#
#  Usage:
#    FORMULA_TEMPLATE=Formula/syncara.rb \
#    VERSION=0.1.0 \
#    MACOS_ARM64_SHA256=... \
#    MACOS_X86_64_SHA256=... \
#    LINUX_ARM64_SHA256=... \
#    LINUX_X86_64_SHA256=... \
#      scripts/generate-formula.sh > syncara.rb
# ─────────────────────────────────────────────────────────────

set -euo pipefail

: "${FORMULA_TEMPLATE:?missing FORMULA_TEMPLATE}"
: "${VERSION:?missing VERSION}"
: "${MACOS_ARM64_SHA256:?missing MACOS_ARM64_SHA256}"
: "${MACOS_X86_64_SHA256:?missing MACOS_X86_64_SHA256}"
: "${LINUX_ARM64_SHA256:?missing LINUX_ARM64_SHA256}"
: "${LINUX_X86_64_SHA256:?missing LINUX_X86_64_SHA256}"

sed \
  -e "s/VERSION_PLACEHOLDER/$VERSION/g" \
  -e "s/MACOS_ARM64_SHA256_PLACEHOLDER/$MACOS_ARM64_SHA256/g" \
  -e "s/MACOS_X86_64_SHA256_PLACEHOLDER/$MACOS_X86_64_SHA256/g" \
  -e "s/LINUX_ARM64_SHA256_PLACEHOLDER/$LINUX_ARM64_SHA256/g" \
  -e "s/LINUX_X86_64_SHA256_PLACEHOLDER/$LINUX_X86_64_SHA256/g" \
  "$FORMULA_TEMPLATE"
