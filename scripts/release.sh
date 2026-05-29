#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
#  Syncara — release helper
#  Usage:
#    scripts/release.sh <version>   # e.g. scripts/release.sh 0.1.0
#
#  Creates and pushes a signed tag, which triggers the
#  .github/workflows/release.yml workflow to build and publish.
# ─────────────────────────────────────────────────────────────

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>"
  echo "  e.g. $0 0.1.0"
  exit 1
fi

VERSION="$1"
TAG="v$VERSION"

# Validate version format.
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
  echo "error: version must be semver (e.g. 0.1.0, 0.1.0-rc.1)"
  exit 1
fi

# Ensure working tree is clean.
if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree is dirty — commit or stash changes first"
  exit 1
fi

# Ensure we are on main.
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" != "main" ]; then
  echo "warning: tagging from branch '$BRANCH', not 'main'"
fi

# Update crate versions.
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" crates/syncara-core/Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" crates/syncara-cli/Cargo.toml

echo ""
echo "Creating release $TAG"
echo ""

git add -A
git commit -m "release v$VERSION"
git tag -s "$TAG" -m "Syncara v$VERSION"

echo ""
echo "Tag $TAG created locally."
echo "To publish, run:"
echo "  git push origin main --tags"
echo ""
echo "This will trigger the release workflow at:"
echo "  https://github.com/anomalyco/syncara/actions/workflows/release.yml"
