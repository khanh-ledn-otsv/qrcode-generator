#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

astro_build_script="$(node -p "require('./package.json').scripts['build:astro']")"
if [[ "${astro_build_script}" != "astro build" ]]; then
  echo "build:astro must use the Node environment prepared by CI; found '${astro_build_script}'." >&2
  exit 1
fi

echo "CI build contract passed."
