#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

required_wasm_pack="wasm-pack 0.15.0"
installed_wasm_pack="$(wasm-pack --version 2>/dev/null || true)"
if [[ "${installed_wasm_pack}" != "${required_wasm_pack}" ]]; then
  cargo install --locked --force wasm-pack --version 0.15.0
fi

wasm-pack build crates/qr-web --target web --release --out-dir ../../src/generated/wasm --out-name qr_web
