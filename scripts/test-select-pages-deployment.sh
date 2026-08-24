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
assert_deployment "true" "--force"
assert_deployment "true" "--force" "README.md" ".github/workflows/correctness.yml"
assert_deployment "true" "__unavailable_ci_base__"
assert_deployment "true" "Cargo.lock"
assert_deployment "true" ".nvmrc"
assert_deployment "true" "astro.config.mjs"
assert_deployment "true" "assets/input.css"
assert_deployment "true" "crates/qr-web/src/wasm_api.rs"
assert_deployment "true" "package.json"
assert_deployment "true" "pnpm-lock.yaml"
assert_deployment "true" "public/favicon.svg"
assert_deployment "true" "scripts/build-web-wasm.sh"
assert_deployment "true" "src/styles/global.css"
assert_deployment "true" "tsconfig.json"

echo "Pages deployment selection passed."
