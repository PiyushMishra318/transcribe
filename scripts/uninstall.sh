#!/usr/bin/env bash
# Uninstall transcribe (Linux / macOS).
set -euo pipefail
PURGE=false
YES=false
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=true ;;
    -y | --yes) YES=true ;;
  esac
done

ARGS=(uninstall -y)
[[ "$PURGE" == true ]] && ARGS+=(--purge)

if command -v transcribe >/dev/null 2>&1; then
  transcribe "${ARGS[@]}"
  exit $?
fi

BIN="${HOME}/.local/bin/transcribe"
[[ -f "$BIN" ]] && rm -f "$BIN" && echo "removed: $BIN"

if [[ "$PURGE" == true && -d "${HOME}/.transcribe" ]]; then
  rm -rf "${HOME}/.transcribe"
  echo "removed: ${HOME}/.transcribe"
fi

echo "done (fallback cleanup)"
