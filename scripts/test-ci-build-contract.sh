#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

astro_build_script="$(node -p "require('./package.json').scripts['build:astro']")"
if [[ "${astro_build_script}" != "astro build" ]]; then
  echo "build:astro must use the Node environment prepared by CI; found '${astro_build_script}'." >&2
  exit 1
fi

workflow_path=".github/workflows/correctness.yml"
for required_path in '      - "AGENTS.md"' '      - "astro.config.mjs"' '      - "docs/TESTING_STRATEGY.md"' '      - "docs/agents/verification.md"' '      - "public/**"' '      - "src/**"' '      - "tsconfig.json"'; do
  if ! grep -Fqx -- "${required_path}" "${workflow_path}"; then
    echo "Correctness workflow is missing a verification-impact path: ${required_path}" >&2
    exit 1
  fi
done

if [[ "$(grep -c '^    timeout-minutes:' "${workflow_path}")" -ne 2 ]]; then
  echo "Every correctness job must have an explicit timeout." >&2
  exit 1
fi

extended_workflow_path=".github/workflows/extended-decoders.yml"
for required_path in '      - "crates/qr-web/**"' '      - "scripts/build-web-wasm.sh"'; do
  if ! grep -Fqx -- "${required_path}" "${extended_workflow_path}"; then
    echo "Extended decoder workflow is missing an evidence-impact path: ${required_path}" >&2
    exit 1
  fi
done

if [[ "$(grep -c '^    timeout-minutes:' "${extended_workflow_path}")" -ne 1 ]]; then
  echo "The extended decoder job must have an explicit timeout." >&2
  exit 1
fi

step_condition() {
  local step_name="$1"
  awk -v step_name="${step_name}" '
    $0 == "      - name: " step_name { in_step = 1; next }
    in_step && /^      - / { exit }
    in_step && /^        if: / { sub(/^        if: /, ""); print; exit }
  ' "${workflow_path}"
}

wasm_cache_condition="$(step_condition "Cache pinned wasm-bindgen tools")"
if [[ "${wasm_cache_condition}" != *"steps.verification.outputs.scope == 'render'"* ]]; then
  echo "The wasm-bindgen tool cache must cover render-scoped verification." >&2
  exit 1
fi

wasm_install_condition="$(step_condition "Install pinned wasm-bindgen tools for focused render verification")"
if [[ "${wasm_install_condition}" != "steps.verification.outputs.scope == 'render'" ]]; then
  echo "Focused render verification must install the pinned wasm-bindgen test runner." >&2
  exit 1
fi

echo "CI build contract passed."
