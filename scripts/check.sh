#!/usr/bin/env bash
# Run the same checks as CI locally (fmt + clippy + test).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> check: cargo fmt --check"
cargo fmt --all -- --check

echo "==> check: cargo clippy"
cargo clippy --locked --no-default-features -- -D warnings

echo "==> check: cargo test"
cargo test --locked --no-default-features

echo "==> check: OK"
