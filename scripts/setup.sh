#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

required_commands=(cargo corepack git node rustup uv)
for command_name in "${required_commands[@]}"; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Missing required command: ${command_name}" >&2
    exit 1
  fi
done

node_major="$(node --version | sed -E 's/^v([0-9]+).*/\1/')"
if [[ "${node_major}" != "24" ]]; then
  echo "Node.js v24 is required; found $(node --version). Run 'nvm use' first." >&2
  exit 1
fi

corepack enable
corepack pnpm install --frozen-lockfile
uv sync --project tests/oracles --locked
rustup target add wasm32-unknown-unknown

if [[ "$(trunk --version 2>/dev/null || true)" != "trunk 0.21.14" ]]; then
  cargo install --locked --force trunk --version 0.21.14
fi
if [[ "$(wasm-bindgen-test-runner --version 2>/dev/null || true)" != "wasm-bindgen-test-runner 0.2.126" ]]; then
  cargo install --locked --force wasm-bindgen-cli --version 0.2.126
fi

corepack pnpm exec playwright install chromium firefox webkit

zxing_source="${repository_root}/tests/oracles/zxing-cpp"
zxing_commit="8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825"
if [[ ! -d "${zxing_source}/.git" ]]; then
  git clone --recurse-submodules https://github.com/zxing-cpp/zxing-cpp.git "${zxing_source}"
  git -C "${zxing_source}" checkout --detach "${zxing_commit}"
  git -C "${zxing_source}" submodule update --init --recursive
else
  actual_commit="$(git -C "${zxing_source}" rev-parse HEAD)"
  if [[ "${actual_commit}" != "${zxing_commit}" ]]; then
    echo "ZXing-C++ must be at ${zxing_commit}; found ${actual_commit}." >&2
    exit 1
  fi
  if [[ -n "$(git -C "${zxing_source}" status --porcelain --untracked-files=no)" ]]; then
    echo "ZXing-C++ has tracked modifications; refusing to build an unpinned oracle." >&2
    exit 1
  fi
fi

uv run --project tests/oracles --locked cmake \
  -S "${zxing_source}" \
  -B "${zxing_source}/build" \
  -DCMAKE_BUILD_TYPE=Release \
  -DZXING_EXAMPLES=ON
uv run --project tests/oracles --locked cmake \
  --build "${zxing_source}/build" \
  --config Release

echo "Setup complete. Run 'pnpm run verify'."
