#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

scope_only="false"
if [[ "${1:-}" == "--scope-only" ]]; then
  scope_only="true"
  shift
fi

changed_paths=()

if [[ "$#" -gt 0 ]]; then
  changed_paths=("$@")
elif [[ -n "${CI_VERIFY_BASE_SHA:-}" ]]; then
  if [[ "${CI_VERIFY_BASE_SHA}" =~ ^0+$ ]] || ! git cat-file -e "${CI_VERIFY_BASE_SHA}^{commit}" 2>/dev/null; then
    changed_paths+=("__unavailable_ci_base__")
  else
    while IFS= read -r changed_path; do
      changed_paths+=("${changed_path}")
    done < <(git diff --name-only "${CI_VERIFY_BASE_SHA}" HEAD)
  fi
elif [[ "${CI:-false}" == "true" ]]; then
  changed_paths+=("__missing_ci_base__")
else
  while IFS= read -r changed_path; do
    changed_paths+=("${changed_path}")
  done < <(
    {
      git diff --name-only HEAD
      git ls-files --others --exclude-standard
    } | sort -u
  )
fi

scope="$(scripts/select-verification.sh "${changed_paths[@]}")"
if [[ "${scope_only}" == "true" ]]; then
  printf '%s\n' "${scope}"
  exit 0
fi

echo "Selected verification scope: ${scope}"

case "${scope}" in
  none)
    echo "No executable checks are required for the changed paths."
    ;;
  meta)
    exec pnpm run verify:meta
    ;;
  python)
    exec pnpm run verify:python
    ;;
  web)
    exec pnpm run verify:web
    ;;
  full)
    exec pnpm run verify
    ;;
  *)
    echo "Unknown verification scope: ${scope}" >&2
    exit 1
    ;;
esac
