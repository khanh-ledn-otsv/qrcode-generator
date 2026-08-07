#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

pnpm run build
uv run --project tests/oracles --locked python tests/support/check_bundle_size.py \
  dist tests/baselines/resources.json \
  --evidence target/release-evidence/bundle-size.json
