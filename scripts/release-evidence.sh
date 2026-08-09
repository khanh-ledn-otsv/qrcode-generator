#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

evidence_dir="target/release-evidence"
mkdir -p "${evidence_dir}"

pnpm run test:approved
pnpm run test:decode
uv run --project tests/oracles --locked python tests/support/collect_approved_output_evidence.py \
  --png "${evidence_dir}/approved-output-png.json" \
  --svg "${evidence_dir}/approved-output-svg.json" \
  --output "${evidence_dir}/approved-output-matrix.json"
pnpm run test:adverse:decode
find dist -maxdepth 1 -type f -print0 \
  | sort -z \
  | xargs -0 shasum -a 256 > "${evidence_dir}/artifact-sha256.txt"
