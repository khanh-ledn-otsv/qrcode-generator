#!/usr/bin/env bash

set -euo pipefail

deploy="false"

for changed_path in "$@"; do
  case "${changed_path}" in
    __unavailable_ci_base__ | __missing_ci_base__)
      deploy="true"
      break
      ;;
    .cargo/* | Cargo.lock | Cargo.toml | Trunk.toml | assets/* | crates/* | rust-toolchain.toml)
      deploy="true"
      break
      ;;
  esac
done

printf '%s\n' "${deploy}"
