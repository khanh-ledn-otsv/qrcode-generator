#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Release readiness evidence requires a clean git worktree." >&2
  exit 1
fi

node_version="$(node --version)"
pnpm_version="$(pnpm --version)"
rustc_version="$(rustc --version)"
trunk_version="$(trunk --version)"
playwright_version="$(pnpm exec playwright --version)"
zxing_source="tests/oracles/zxing-cpp"
zxing_version="$(git -C "${zxing_source}" rev-parse HEAD)"
release_candidate="$(git rev-parse HEAD)"

[[ "${node_version}" == v24.* ]] || { echo "Node.js v24 is required." >&2; exit 1; }
[[ "${pnpm_version}" == "11.20.0" ]] || { echo "pnpm 11.20.0 is required." >&2; exit 1; }
[[ "${rustc_version}" == "rustc 1.97.1 "* ]] || { echo "Rust 1.97.1 is required." >&2; exit 1; }
[[ "${trunk_version}" == "trunk 0.21.14" ]] || { echo "Trunk 0.21.14 is required." >&2; exit 1; }
[[ "${playwright_version}" == "Version 1.62.1" ]] || { echo "Playwright 1.62.1 is required." >&2; exit 1; }
[[ "${zxing_version}" == "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825" ]] || { echo "ZXing-C++ is not pinned." >&2; exit 1; }

evidence_root="target/release-readiness"
mkdir -p "${evidence_root}"
first_target="$(mktemp -d "${evidence_root}/build-a.XXXXXX")"
second_target="$(mktemp -d "${evidence_root}/build-b.XXXXXX")"

NO_COLOR=true CARGO_TARGET_DIR="${first_target}" trunk build --release --dist "${first_target}/dist"
NO_COLOR=true CARGO_TARGET_DIR="${second_target}" trunk build --release --dist "${second_target}/dist"

PLAYWRIGHT_JSON_OUTPUT_FILE="${evidence_root}/playwright.json" pnpm exec playwright test --reporter=json
pnpm run release:evidence

uv run --project tests/oracles --locked python tests/support/collect_release_readiness.py \
  --first-build "${first_target}/dist" \
  --second-build "${second_target}/dist" \
  --release-candidate "${release_candidate}" \
  --playwright-report "${evidence_root}/playwright.json" \
  --release-evidence "target/release-evidence" \
  --node "${node_version}" \
  --pnpm "${pnpm_version}" \
  --rustc "${rustc_version}" \
  --trunk "${trunk_version}" \
  --playwright "${playwright_version}" \
  --zxing "${zxing_version}" \
  --output "${evidence_root}/automated.json"

uv run --project tests/oracles --locked python tests/support/validate_release_readiness.py \
  --automated "${evidence_root}/automated.json" \
  --manual "${QR_MANUAL_EVIDENCE:-tests/release/manual-evidence.json}" \
  --output "${evidence_root}/readiness-report.json"
