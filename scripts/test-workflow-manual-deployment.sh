#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
workflow_path="${repository_root}/.github/workflows/correctness.yml"

required_lines=(
  "CI_MANUAL_DISPATCH: \${{ github.event_name == 'workflow_dispatch' }}"
  'verification_scope="full"'
  'deploy_pages="$(scripts/select-pages-deployment.sh --force)"'
)

for required_line in "${required_lines[@]}"; do
  if ! grep -Fq "${required_line}" "${workflow_path}"; then
    echo "Correctness workflow is missing the manual deployment contract: ${required_line}" >&2
    exit 1
  fi
done

echo "Manual workflow deployment contract passed."
