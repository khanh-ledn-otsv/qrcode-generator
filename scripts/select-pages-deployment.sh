#!/usr/bin/env bash

set -euo pipefail

deploy="false"

if [[ "${1:-}" == "--force" ]]; then
  printf 'true\n'
  exit 0
fi

for changed_path in "$@"; do
  case "${changed_path}" in
    __unavailable_ci_base__ | __missing_ci_base__)
      deploy="true"
      break
      ;;
    .cargo/* | .nvmrc | Cargo.lock | Cargo.toml | assets/* | astro.config.mjs | crates/* | package.json | pnpm-lock.yaml | public/* | rust-toolchain.toml | scripts/build-web-wasm.sh | src/* | tsconfig.json)
      deploy="true"
      break
      ;;
  esac
done

printf '%s\n' "${deploy}"
