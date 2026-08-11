#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

required_commands=(cc git make uv)
for command_name in "${required_commands[@]}"; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Missing required command: ${command_name}" >&2
    exit 1
  fi
done

zxing_source="${repository_root}/tests/oracles/zxing-cpp"
zxing_commit="8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825"
if [[ ! -d "${zxing_source}/.git" ]]; then
  git clone --recurse-submodules https://github.com/zxing-cpp/zxing-cpp.git "${zxing_source}"
  git -C "${zxing_source}" checkout --detach "${zxing_commit}"
else
  actual_commit="$(git -C "${zxing_source}" rev-parse HEAD)"
  if [[ "${actual_commit}" != "${zxing_commit}" ]]; then
    echo "ZXing-C++ must be at ${zxing_commit}; found ${actual_commit}." >&2
    exit 1
  fi
fi
git -C "${zxing_source}" submodule sync --recursive
git -C "${zxing_source}" submodule update --init --recursive
if [[ -n "$(git -C "${zxing_source}" status --porcelain --untracked-files=no)" ]]; then
  echo "ZXing-C++ has tracked modifications; refusing to build an unpinned oracle." >&2
  exit 1
fi
if git -C "${zxing_source}" submodule status --recursive | grep -Eq '^[+-U]'; then
  echo "ZXing-C++ has an uninitialized or mismatched submodule." >&2
  exit 1
fi

uv run --project tests/oracles --locked cmake \
  -S "${zxing_source}" \
  -B "${zxing_source}/build" \
  -DCMAKE_BUILD_TYPE=Release \
  -DZXING_EXAMPLES=ON
uv run --project tests/oracles --locked cmake \
  --build "${zxing_source}/build" \
  --config Release

quirc_source="${repository_root}/tests/oracles/quirc"
quirc_commit="542848dd6b9b0eaa9587bbf25b9bc67bd8a71fca"
if [[ ! -d "${quirc_source}/.git" ]]; then
  git clone https://github.com/dlbeer/quirc.git "${quirc_source}"
  git -C "${quirc_source}" checkout --detach "${quirc_commit}"
else
  actual_commit="$(git -C "${quirc_source}" rev-parse HEAD)"
  if [[ "${actual_commit}" != "${quirc_commit}" ]]; then
    echo "quirc must be at ${quirc_commit}; found ${actual_commit}." >&2
    exit 1
  fi
  if [[ -n "$(git -C "${quirc_source}" status --porcelain --untracked-files=no)" ]]; then
    echo "quirc has tracked modifications; refusing to build an unpinned oracle." >&2
    exit 1
  fi
fi
make -C "${quirc_source}" SDL_CFLAGS= libquirc.a
cc -std=c11 -Wall -Wextra -Werror \
  -I"${quirc_source}/lib" \
  "${repository_root}/tests/oracles/quirc-reader.c" \
  "${quirc_source}/libquirc.a" \
  -lm \
  -o "${repository_root}/tests/oracles/quirc-reader"

echo "Decoder setup complete."
