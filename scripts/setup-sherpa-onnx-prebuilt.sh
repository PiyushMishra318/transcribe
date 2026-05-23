#!/usr/bin/env bash
# Download and extract sherpa-onnx shared prebuilt libs into target/sherpa-onnx-prebuilt/.
# Matches sherpa-onnx-sys build.rs archive names (shared / CPU release builds).
# SHERPA_ONNX_LIB_DIR must be absolute: dependency build.rs resolves relative paths from the crate dir.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${SHERPA_ONNX_VERSION:-$(
  awk '/^name = "sherpa-onnx-sys"$/{getline; if ($1 == "version") { gsub(/"/, "", $3); print $3; exit }}' Cargo.lock
)}"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  if [[ "${CARGO_TARGET_DIR}" == /* ]]; then
    TARGET_DIR="${CARGO_TARGET_DIR}"
  else
    TARGET_DIR="${ROOT}/${CARGO_TARGET_DIR}"
  fi
else
  TARGET_DIR="${ROOT}/target"
fi

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

sherpa_lib_ready() {
  local dir="$1"
  [[ -d "${dir}" ]] || return 1
  case "${OS}" in
    Linux)
      find "${dir}" -maxdepth 1 \( -name '*.so' -o -name '*.so.*' \) -print -quit | grep -q .
      ;;
    Darwin)
      find "${dir}" -maxdepth 1 -name '*.dylib' -print -quit | grep -q .
      ;;
    *)
      return 1
      ;;
  esac
}

if sherpa_lib_ready "${LIB_DIR}"; then
  echo "sherpa-onnx prebuilt already present: ${LIB_DIR}"
else
  echo "Installing sherpa-onnx prebuilt (${STEM})"
  rm -rf "${CACHE_ROOT}/${STEM}"
  mkdir -p "${CACHE_ROOT}"
  URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/v${VERSION}/${ARCHIVE}"
  echo "Downloading ${URL}"
  curl -fsSL -o "${CACHE_ROOT}/${ARCHIVE}" "${URL}"
  tar -xjf "${CACHE_ROOT}/${ARCHIVE}" -C "${CACHE_ROOT}"
  if ! sherpa_lib_ready "${LIB_DIR}"; then
    echo "error: expected lib directory missing or empty after extract: ${LIB_DIR}" >&2
    echo "cache root contents:" >&2
    ls -la "${CACHE_ROOT}" >&2 || true
    exit 1
  fi
  echo "Extracted to ${CACHE_ROOT}/${STEM}"
fi

LIB_DIR="$(cd "${LIB_DIR}" && pwd)"

if ! sherpa_lib_ready "${LIB_DIR}"; then
  echo "error: sherpa lib directory is not ready: ${LIB_DIR}" >&2
  ls -la "${LIB_DIR}" >&2 || true
  exit 1
fi

echo "sherpa-onnx libs in ${LIB_DIR}:"
ls -la "${LIB_DIR}"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "SHERPA_ONNX_LIB_DIR=${LIB_DIR}" >>"${GITHUB_ENV}"
fi

export SHERPA_ONNX_LIB_DIR="${LIB_DIR}"
echo "SHERPA_ONNX_LIB_DIR=${SHERPA_ONNX_LIB_DIR}"

# Link-time rpath points at the prebuilt tree; debug smoke tests and assert_cmd need the loader path too
# (rust-cache can restore target/debug/transcribe without colocated .so copies).
case "${OS}" in
  Linux)
    if [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
      export LD_LIBRARY_PATH="${LIB_DIR}:${LD_LIBRARY_PATH}"
    else
      export LD_LIBRARY_PATH="${LIB_DIR}"
    fi
    if [[ -n "${GITHUB_ENV:-}" ]]; then
      echo "LD_LIBRARY_PATH=${LD_LIBRARY_PATH}" >>"${GITHUB_ENV}"
    fi
    echo "LD_LIBRARY_PATH=${LD_LIBRARY_PATH}"
    ;;
  Darwin)
    if [[ -n "${DYLD_LIBRARY_PATH:-}" ]]; then
      export DYLD_LIBRARY_PATH="${LIB_DIR}:${DYLD_LIBRARY_PATH}"
    else
      export DYLD_LIBRARY_PATH="${LIB_DIR}"
    fi
    if [[ -n "${GITHUB_ENV:-}" ]]; then
      echo "DYLD_LIBRARY_PATH=${DYLD_LIBRARY_PATH}" >>"${GITHUB_ENV}"
    fi
    echo "DYLD_LIBRARY_PATH=${DYLD_LIBRARY_PATH}"
    ;;
esac
