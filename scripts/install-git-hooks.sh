#!/usr/bin/env bash
# Opt-in: point this repo at .githooks/ (local config only, not global).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/pre-push scripts/pre-push.sh scripts/check.sh scripts/install-git-hooks.sh

echo "==> Git hooks installed for this repository"
echo "    core.hooksPath = .githooks"
echo "    pre-push runs:  ./scripts/pre-push.sh (fmt --check + clippy)"
echo ""
echo "Full local CI parity before push:"
echo "    ./scripts/check.sh"
echo ""
echo "To disable hooks for one push:"
echo "    git push --no-verify"
