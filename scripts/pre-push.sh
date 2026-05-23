#!/usr/bin/env bash
# Fast checks before git push (fmt + clippy). Used by .githooks/pre-push.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> pre-push: sherpa-onnx prebuilt libs"
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    pwsh -NoProfile -File scripts/setup-sherpa-onnx-prebuilt.ps1
    ;;
  *)
    bash scripts/setup-sherpa-onnx-prebuilt.sh
    ;;
esac

echo "==> pre-push: cargo fmt --check"
cargo fmt --all -- --check

echo "==> pre-push: cargo clippy"
cargo clippy --locked --no-default-features -- -D warnings

echo "==> pre-push: OK"
