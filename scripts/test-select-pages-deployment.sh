#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
selector="${repository_root}/scripts/select-pages-deployment.sh"

assert_deployment() {
  local expected="$1"
  shift
  local actual

  actual="$(${selector} "$@")"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "Expected deployment '${expected}' for '$*'; found '${actual}'." >&2
    exit 1
  fi
}

assert_deployment "false"
assert_deployment "false" "README.md" "tests/support/test_check_coverage_report.py"
assert_deployment "false" ".github/workflows/correctness.yml"
assert_deployment "false" ".github/workflows/extended-decoders.yml"
assert_deployment "true" "__unavailable_ci_base__"
assert_deployment "true" "Cargo.lock"
assert_deployment "true" "assets/input.css"
assert_deployment "true" "crates/qr-web/src/main.rs"

echo "Pages deployment selection passed."
