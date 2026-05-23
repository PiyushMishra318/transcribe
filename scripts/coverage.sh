#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo llvm-cov --no-default-features --release --fail-under-lines 100 -- --include-ignored "$@"
