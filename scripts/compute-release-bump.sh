#!/usr/bin/env bash
# Determine semver bump since the last v* tag (used by .github/workflows/auto-release.yml).
# Writes bump_level, new_version, tag_name, prerelease to GITHUB_OUTPUT when set.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

strip_prerelease_suffix() {
  local v="$1"
  v="${v#v}"
  v="${v%-alpha}"
  v="${v%-beta}"
  v="${v%-rc.*}"
  echo "$v"
}

parse_cargo_version() {
  awk -F'"' '/^version = / { print $2; exit }' Cargo.toml
}

LAST_TAG=""
if git describe --tags --match 'v*' --abbrev=0 >/dev/null 2>&1; then
  LAST_TAG="$(git describe --tags --match 'v*' --abbrev=0)"
fi

if [[ -n "$LAST_TAG" ]]; then
  if git merge-base --is-ancestor "$LAST_TAG" HEAD 2>/dev/null; then
    RANGE="${LAST_TAG}..HEAD"
  else
    RANGE="HEAD"
  fi
  BASE_VERSION="$(strip_prerelease_suffix "$LAST_TAG")"
else
  RANGE="HEAD"
  BASE_VERSION="$(parse_cargo_version)"
fi

# Nothing new since last tag (e.g. re-run on tagged commit).
if [[ -n "$LAST_TAG" ]] && [[ -z "$(git rev-list "${RANGE}" 2>/dev/null || true)" ]]; then
  echo "No commits since ${LAST_TAG}; nothing to release." >&2
  exit 2
fi

bump_level="patch"

commit_subjects() {
  if [[ -n "$LAST_TAG" ]]; then
    git log "${LAST_TAG}..HEAD" --pretty=format:%s
  else
    git log HEAD --pretty=format:%s
  fi
}

commit_bodies() {
  if [[ -n "$LAST_TAG" ]]; then
    git log "${LAST_TAG}..HEAD" --pretty=format:%b
  else
    git log HEAD --pretty=format:%b
  fi
}

SUBJECTS="$(commit_subjects || true)"
BODIES="$(commit_bodies || true)"

# --- Conventional commits (highest signal first) ---
if printf '%s\n%s\n' "$SUBJECTS" "$BODIES" | grep -qiE '(^|[[:space:]])BREAKING[[:space:]]CHANGE'; then
  bump_level="major"
elif printf '%s\n' "$SUBJECTS" | grep -qE '^[a-zA-Z]+(\([^)]+\))?!:'; then
  bump_level="major"
elif printf '%s\n' "$SUBJECTS" | grep -qE '^feat(\([^)]+\))?:'; then
  bump_level="minor"
elif printf '%s\n' "$SUBJECTS" | grep -qE '^fix(\([^)]+\))?:'; then
  bump_level="patch"
fi

# --- File paths (when commits are non-conventional) ---
if [[ -n "$LAST_TAG" ]]; then
  FILES="$(git diff --name-only "${LAST_TAG}" HEAD)"
else
  FILES="$(git diff --name-only --cached HEAD 2>/dev/null || git ls-files)"
fi

only_non_feature_paths() {
  local f
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    case "$f" in
      .github/* | docs/* | *.md | LICENSE | CODE_OF_CONDUCT.md | CONTRIBUTING.md | Cargo.lock | scripts/compute-release-bump.sh)
        continue
        ;;
      *)
        return 1
        ;;
    esac
  done <<<"$FILES"
  return 0
}

subjects_are_maintenance_only() {
  local s
  if printf '%s\n%s\n' "$SUBJECTS" "$BODIES" | grep -qiE '(^|[[:space:]])BREAKING[[:space:]]CHANGE'; then
    return 1
  fi
  while IFS= read -r s; do
    [[ -z "$s" ]] && continue
    if printf '%s\n' "$s" | grep -qE '^feat(\([^)]+\))?!:'; then
      return 1
    fi
    if printf '%s\n' "$s" | grep -qE '^[a-zA-Z]+(\([^)]+\))?!:'; then
      return 1
    fi
    if printf '%s\n' "$s" | grep -qE '^feat(\([^)]+\))?:'; then
      return 1
    fi
  done <<<"$SUBJECTS"
  return 0
}

if [[ "$bump_level" == "patch" ]] && printf '%s\n' "$FILES" | grep -qE '^src/'; then
  if printf '%s\n' "$SUBJECTS" | grep -qE '^feat(\([^)]+\))?:'; then
    bump_level="minor"
  elif printf '%s\n' "$SUBJECTS" | grep -qE '^(fix|chore|docs|ci|build|test)(\([^)]+\))?:'; then
    bump_level="patch"
  elif printf '%s\n' "$SUBJECTS" | grep -qE '^Fix '; then
    bump_level="patch"
  elif subjects_are_maintenance_only; then
    bump_level="patch"
  else
    bump_level="minor"
  fi
fi

if [[ "$bump_level" == "patch" ]] && only_non_feature_paths; then
  bump_level="patch"
fi

IFS='.' read -r MAJOR MINOR PATCH <<<"$BASE_VERSION"
MAJOR="${MAJOR:-0}"
MINOR="${MINOR:-0}"
PATCH="${PATCH:-0}"

case "$bump_level" in
  major)
    MAJOR=$((MAJOR + 1))
    MINOR=0
    PATCH=0
    ;;
  minor)
    MINOR=$((MINOR + 1))
    PATCH=0
    ;;
  patch)
    PATCH=$((PATCH + 1))
    ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"

if [[ "$MAJOR" -eq 0 ]]; then
  TAG_NAME="v${NEW_VERSION}-alpha"
  PRERELEASE="true"
else
  TAG_NAME="v${NEW_VERSION}"
  PRERELEASE="false"
fi

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "bump_level=${bump_level}"
    echo "new_version=${NEW_VERSION}"
    echo "tag_name=${TAG_NAME}"
    echo "prerelease=${PRERELEASE}"
    echo "last_tag=${LAST_TAG}"
  } >>"$GITHUB_OUTPUT"
else
  echo "bump_level=${bump_level}"
  echo "new_version=${NEW_VERSION}"
  echo "tag_name=${TAG_NAME}"
  echo "prerelease=${PRERELEASE}"
  echo "last_tag=${LAST_TAG}"
fi
