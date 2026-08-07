#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

evidence_dir="target/release-evidence/coverage"
mkdir -p "${evidence_dir}"

cargo llvm-cov --package qr-core --all-features --json --summary-only \
  --output-path "${evidence_dir}/qr-core.json"
uv run --project tests/oracles --locked python tests/support/check_coverage_report.py \
  "${evidence_dir}/qr-core.json" --minimum-lines 95 --minimum-regions 90 \
  --evidence "${evidence_dir}/qr-core-threshold.json"
uv run --project tests/oracles --locked python tests/support/check_coverage_report.py \
  "${evidence_dir}/qr-core.json" --minimum-lines 98 --minimum-regions 95 \
  --include crates/qr-core/src/reed_solomon.rs \
  --include crates/qr-core/src/matrix.rs \
  --include crates/qr-core/src/selection.rs \
  --include crates/qr-core/src/bch.rs \
  --include crates/qr-core/src/penalty.rs \
  --evidence "${evidence_dir}/qr-core-critical-threshold.json"

cargo llvm-cov --package qr-render --all-features --json --summary-only \
  --output-path "${evidence_dir}/qr-render.json"
uv run --project tests/oracles --locked python tests/support/check_coverage_report.py \
  "${evidence_dir}/qr-render.json" --minimum-lines 90 --minimum-regions 85 \
  --evidence "${evidence_dir}/qr-render-threshold.json"
uv run --project tests/oracles --locked python tests/support/check_coverage_report.py \
  "${evidence_dir}/qr-render.json" --minimum-lines 98 --minimum-regions 95 \
  --include crates/qr-render/src/profile.rs \
  --include crates/qr-render/src/geometry.rs \
  --evidence "${evidence_dir}/qr-render-geometry-threshold.json"

cargo llvm-cov --package qr-web --all-features --json --summary-only \
  --output-path "${evidence_dir}/qr-web.json"
uv run --project tests/oracles --locked python tests/support/check_coverage_report.py \
  "${evidence_dir}/qr-web.json" --minimum-lines 85 --minimum-regions 80 \
  --include crates/qr-web/src/workflow.rs \
  --evidence "${evidence_dir}/qr-web-state-threshold.json"
