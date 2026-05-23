#!/usr/bin/env bash
# Install the `transcribe` CLI for the current user (Linux / macOS).
# Usage: ./scripts/install.sh [--cpu-only]

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CPU_ONLY=false
for arg in "$@"; do
  case "$arg" in
    --cpu-only) CPU_ONLY=true ;;
  esac
done

echo "==> Episode Transcribe installer"
echo "    Source: $ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: Rust (cargo) not found. Install from https://rustup.rs/" >&2
  exit 1
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "warning: ffmpeg not found on PATH (required at runtime)" >&2
fi

echo "==> Building release binary..."
if [[ "$CPU_ONLY" == true ]]; then
  cargo build --release --locked --no-default-features
else
  cargo build --release --locked
fi

BIN="$ROOT/target/release/transcribe"
[[ -f "$BIN" ]] || { echo "error: build failed — $BIN missing" >&2; exit 1; }

PREFIX="${PREFIX:-$HOME/.local}"
INSTALL_DIR="$PREFIX/bin"
mkdir -p "$INSTALL_DIR"
cp "$BIN" "$INSTALL_DIR/transcribe"
chmod +x "$INSTALL_DIR/transcribe"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo "==> Add to your shell profile (~/.bashrc or ~/.zshrc):"
    echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

"$INSTALL_DIR/transcribe" --version
echo "==> Installed to $INSTALL_DIR/transcribe"
echo ""
echo "Next:"
echo "  download models → models/README.md"
echo "  transcribe help"
echo "  transcribe project init my-campaign"
echo "  transcribe profiles build && transcribe profiles label"
echo "  transcribe run ."
echo "Uninstall: transcribe uninstall -y  (or scripts/uninstall.sh)"
