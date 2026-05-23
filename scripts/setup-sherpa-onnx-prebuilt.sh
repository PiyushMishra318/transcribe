#!/usr/bin/env bash
# Download and extract sherpa-onnx shared prebuilt libs into target/sherpa-onnx-prebuilt/.
# Matches sherpa-onnx-sys build.rs archive names (shared / CPU release builds).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${SHERPA_ONNX_VERSION:-$(
  awk '/^name = "sherpa-onnx-sys"$/{getline; if ($1 == "version") { gsub(/"/, "", $3); print $3; exit }}' Cargo.lock
)}"

TARGET_DIR="${CARGO_TARGET_DIR:-target}"
CACHE_ROOT="${TARGET_DIR}/sherpa-onnx-prebuilt"

OS="$(uname -s)"
ARCH="$(uname -m)"
case "${OS}/${ARCH}" in
  Linux/x86_64) ARCHIVE="sherpa-onnx-v${VERSION}-linux-x64-shared-lib.tar.bz2" ;;
  Linux/aarch64 | Linux/arm64) ARCHIVE="sherpa-onnx-v${VERSION}-linux-aarch64-shared-cpu-lib.tar.bz2" ;;
  Darwin/arm64) ARCHIVE="sherpa-onnx-v${VERSION}-osx-arm64-shared-lib.tar.bz2" ;;
  Darwin/x86_64) ARCHIVE="sherpa-onnx-v${VERSION}-osx-x64-shared-lib.tar.bz2" ;;
  *)
    echo "error: unsupported platform ${OS}/${ARCH} for sherpa-onnx prebuilt download" >&2
    exit 1
    ;;
esac

STEM="${ARCHIVE%.tar.bz2}"
LIB_DIR="${CACHE_ROOT}/${STEM}/lib"

if [[ -d "${LIB_DIR}" ]]; then
  echo "sherpa-onnx prebuilt already present: ${LIB_DIR}"
else
  mkdir -p "${CACHE_ROOT}"
  URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/v${VERSION}/${ARCHIVE}"
  echo "Downloading ${URL}"
  curl -fsSL -o "${CACHE_ROOT}/${ARCHIVE}" "${URL}"
  tar -xjf "${CACHE_ROOT}/${ARCHIVE}" -C "${CACHE_ROOT}"
  if [[ ! -d "${LIB_DIR}" ]]; then
    echo "error: expected lib directory missing after extract: ${LIB_DIR}" >&2
    exit 1
  fi
  echo "Extracted to ${CACHE_ROOT}/${STEM}"
fi

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "SHERPA_ONNX_LIB_DIR=${LIB_DIR}" >>"${GITHUB_ENV}"
fi

export SHERPA_ONNX_LIB_DIR="${LIB_DIR}"
echo "SHERPA_ONNX_LIB_DIR=${SHERPA_ONNX_LIB_DIR}"
