#!/usr/bin/env bash

set -euo pipefail

scope="none"

merge_scope() {
  local candidate="$1"

  if [[ "${scope}" == "none" || "${scope}" == "${candidate}" ]]; then
    scope="${candidate}"
  elif [[ "${candidate}" == "none" ]]; then
    return
  else
    scope="full"
  fi
}

for changed_path in "$@"; do
  case "${changed_path}" in
    AGENTS.md | docs/TESTING_STRATEGY.md | docs/agents/verification.md)
      merge_scope "meta"
      ;;
    README.md | docs/* | .scratch/*)
      ;;
    .github/* | scripts/check-doc-links.mjs | scripts/select-verification.sh | scripts/test-select-verification.sh | scripts/verify-changed.sh)
      merge_scope "meta"
      ;;
    ruff.toml | ty.toml | tests/support/*)
      merge_scope "python"
      ;;
    crates/qr-core/*)
      merge_scope "core"
      ;;
    crates/qr-render/*)
      merge_scope "render"
      ;;
    .oxfmtrc.json | .oxlintrc.json | crates/qr-web/* | e2e/* | playwright.config.ts)
      merge_scope "web"
      ;;
    *)
      scope="full"
      break
      ;;
  esac
done

printf '%s\n' "${scope}"
