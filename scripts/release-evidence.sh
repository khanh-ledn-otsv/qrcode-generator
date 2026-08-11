#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

evidence_dir="target/release-evidence"
mkdir -p "${evidence_dir}"

dist_dir="dist"
if [[ "$#" -gt 0 ]]; then
  if [[ "$#" -ne 2 || "$1" != "--dist" ]]; then
    echo "Usage: $0 [--dist existing-dist-directory]" >&2
    exit 1
  fi
  dist_dir="$2"
  if [[ ! -f "${dist_dir}/index.html" ]]; then
    echo "Existing release dist is missing index.html: ${dist_dir}" >&2
    exit 1
  fi
else
  pnpm run build
fi

pnpm run test:approved
pnpm run test:decode
uv run --project tests/oracles --locked python tests/support/collect_approved_output_evidence.py \
  --png "${evidence_dir}/approved-output-png.json" \
  --svg "${evidence_dir}/approved-output-svg.json" \
  --output "${evidence_dir}/approved-output-matrix.json"
pnpm run test:adverse:decode
(
  cd "${dist_dir}"
  find . -maxdepth 1 -type f -print0 \
    | sort -z \
    | xargs -0 shasum -a 256 \
    | sed 's#  \./#  dist/#'
) > "${repository_root}/${evidence_dir}/artifact-sha256.txt"
