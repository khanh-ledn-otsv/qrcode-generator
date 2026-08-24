#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
setup_uv_pin="astral-sh/setup-uv@c771a70e6277c0a99b617c7a806ffedaca235ff9 # v9.0.0"

for workflow in correctness.yml extended-decoders.yml; do
  workflow_path="${repository_root}/.github/workflows/${workflow}"
  if ! grep -Fq "uses: ${setup_uv_pin}" "${workflow_path}"; then
    echo "${workflow} must use the resolvable setup-uv v9.0.0 commit pin." >&2
    exit 1
  fi

  checkout_count="$(grep -c 'uses: actions/checkout@' "${workflow_path}")"
  credential_isolation_count="$(grep -c 'persist-credentials: false' "${workflow_path}")"
  if [[ "${credential_isolation_count}" -ne "${checkout_count}" ]]; then
    echo "${workflow} must disable persisted credentials for every checkout." >&2
    exit 1
  fi
done

echo "Workflow action pins passed."
