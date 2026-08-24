#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
selector="${repository_root}/scripts/select-verification.sh"

assert_scope() {
  local expected="$1"
  shift
  local actual

  actual="$(${selector} "$@")"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "Expected scope '${expected}' for '$*'; found '${actual}'." >&2
    exit 1
  fi
}

assert_scope "none"
assert_scope "none" "README.md" "docs/DEVELOPMENT_PLAN.md"
assert_scope "meta" "AGENTS.md"
assert_scope "meta" "docs/TESTING_STRATEGY.md"
assert_scope "meta" "docs/agents/verification.md"
assert_scope "meta" ".github/workflows/correctness.yml"
assert_scope "meta" "scripts/select-verification.sh"
assert_scope "meta" "scripts/test-ci-build-contract.sh"
assert_scope "meta" "scripts/test-workflow-manual-deployment.sh"
assert_scope "meta" "scripts/check-doc-links.mjs"
assert_scope "python" "tests/support/test_check_coverage_report.py"
assert_scope "core" "crates/qr-core/src/lib.rs"
assert_scope "render" "crates/qr-render/src/lib.rs"
assert_scope "web" "crates/qr-web/src/main.rs" "e2e/workflow.spec.ts"
assert_scope "full" "crates/qr-core/src/lib.rs" "crates/qr-render/src/lib.rs"
assert_scope "full" "crates/qr-render/src/lib.rs" "crates/qr-web/src/main.rs"
assert_scope "full" "Cargo.lock"
assert_scope "full" "unknown-file"
assert_scope "full" "crates/qr-web/src/main.rs" "tests/support/test_check_coverage_report.py"

scope="$(env -u CI_VERIFY_BASE_SHA CI=true "${repository_root}/scripts/verify-changed.sh" --scope-only)"
if [[ "${scope}" != "full" ]]; then
  echo "A CI run without a base commit must select the full gate; found '${scope}'." >&2
  exit 1
fi

echo "Verification scope selection passed."
