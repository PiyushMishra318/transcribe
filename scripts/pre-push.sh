#!/usr/bin/env bash
# Fast checks before git push (fmt + clippy). Used by .githooks/pre-push.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> pre-push: cargo fmt --check"
cargo fmt --all -- --check

echo "==> pre-push: cargo clippy"
cargo clippy --locked --no-default-features -- -D warnings

echo "==> pre-push: OK"
