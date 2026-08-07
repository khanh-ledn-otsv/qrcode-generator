#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

evidence_dir="target/release-evidence/mutation"
mkdir -p "${evidence_dir}"

run_mutants() {
  local output_dir="$1"
  shift
  set +e
  cargo mutants --output "${output_dir}" "$@"
  local mutants_status=$?
  set -e
  if [[ ! -f "${output_dir}/outcomes.json" ]]; then
    echo "cargo-mutants failed before producing outcomes (status ${mutants_status})" >&2
    exit "${mutants_status}"
  fi
}

run_mutants "${evidence_dir}/qr-core" --package qr-core
uv run --project tests/oracles --locked python tests/support/check_mutation_score.py \
  "${evidence_dir}/qr-core" --minimum 85 \
  --evidence "${evidence_dir}/qr-core-threshold.json"
uv run --project tests/oracles --locked python tests/support/check_mutation_score.py \
  "${evidence_dir}/qr-core" --minimum 90 \
  --include src/reed_solomon.rs --include src/matrix.rs --include src/selection.rs \
  --include src/bch.rs --include src/penalty.rs \
  --evidence "${evidence_dir}/qr-core-critical-threshold.json"

run_mutants "${evidence_dir}/qr-render-geometry" --package qr-render \
  --file crates/qr-render/src/profile.rs --file crates/qr-render/src/geometry.rs
uv run --project tests/oracles --locked python tests/support/check_mutation_score.py \
  "${evidence_dir}/qr-render-geometry" --minimum 90 \
  --evidence "${evidence_dir}/qr-render-geometry-threshold.json"
