#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repository_root}"

child_pids=()
child_labels=()

terminate_children() {
  local pid

  for pid in "${child_pids[@]}"; do
    kill "${pid}" 2>/dev/null || true
  done
}

trap terminate_children INT TERM

start_job() {
  local label="$1"
  shift

  "$@" &
  child_pids+=("$!")
  child_labels+=("${label}")
}

wait_for_jobs() {
  local failed=0
  local index

  for ((index = 0; index < ${#child_pids[@]}; index += 1)); do
    if ! wait "${child_pids[index]}"; then
      echo "Verification lane failed: ${child_labels[index]}" >&2
      failed=1
    fi
  done

  child_pids=()
  child_labels=()

  if [[ "${failed}" -ne 0 ]]; then
    return 1
  fi
}

run_compiled_tests() {
  pnpm run test:rust
  pnpm run test:wasm
}

echo "Running independent static checks in parallel..."
start_job "Rust checks" pnpm run check:rust
start_job "web checks" pnpm run check:web
start_job "Python checks" pnpm run check:python
wait_for_jobs

echo "Building the release application once for verification and browser tests..."
pnpm run build

if [[ "${CI:-false}" == "true" ]]; then
  echo "Running compiled, Python, and browser tests serially in CI..."
  run_compiled_tests
  pnpm run test:python
  pnpm run test:e2e:dist
else
  echo "Running compiled, Python, and browser tests in parallel..."
  start_job "native and WASM tests" run_compiled_tests
  start_job "Python tests" pnpm run test:python
  start_job "browser tests" pnpm run test:e2e:dist
  wait_for_jobs
fi

echo "Verification completed successfully."
