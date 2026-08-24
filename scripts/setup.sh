#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

required_commands=(cargo corepack node rustup uv)
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
if [[ "$(wasm-bindgen-test-runner --version 2>/dev/null || true)" != "wasm-bindgen-test-runner 0.2.127" ]]; then
  cargo install --locked --force wasm-bindgen-cli --version 0.2.127
fi

case "${QR_PLAYWRIGHT_INSTALL_MODE:-browser-only}" in
  browser-only)
    corepack pnpm exec playwright install chromium
    ;;
  with-deps)
    corepack pnpm exec playwright install --with-deps chromium
    ;;
  skip)
    ;;
  *)
    echo "QR_PLAYWRIGHT_INSTALL_MODE must be browser-only, with-deps, or skip." >&2
    exit 1
    ;;
esac

"${repository_root}/scripts/setup-decoders.sh" "${QR_DECODER_SETUP_MODE:-all}"

echo "Setup complete. Run 'pnpm run verify'."
