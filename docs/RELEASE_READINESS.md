# Release readiness

This runbook is the repository-owned automated release gate. Manual product checks are performed separately and are not collected as evidence.

## Automated evidence

From a clean worktree, run setup and the readiness gate:

```sh
./scripts/setup.sh
pnpm run release:readiness
```

The command verifies pinned tool versions, produces two release builds in separate Cargo target directories, compares every application artifact by SHA-256, runs desktop Chromium with Playwright retries disabled, and validates the 436-row dual-format approved-output matrix plus all 39 declared adverse-decoder outcomes. It writes the machine evidence and final report under `target/release-readiness/`.

## Acceptance-criterion map

| Criterion | Evidence | Gate |
|---|---|---|
| No runtime payload or logo requests | Playwright privacy test in Chromium | `automated.network_inspection` |
| Clean pinned reproducible build | tool capture plus two artifact hash maps | `automated.reproducible_builds` |
| Chromium critical paths, downloads, and privacy with zero retries | desktop Chromium with `retries: 0` | `automated.browsers`, `automated.downloads` |
| All approved payload/version paths, typed geometry rejections, and adverse decoding | generated PNG/SVG hashes, geometry facts, and pinned-decoder evidence | `automated.artifact_evidence` |
| SVG-first, sizing, per-variant ASCII Byte-mode capacity and selection guidance, Adaptive sizing/logo limitations, transparent/logo, and environment guidance | browser assertion against the visible semantic guide | `automated.guidance` |

The validator rejects missing tests, retry-based browser results, mismatched builds, invalid hashes, runtime network requests, or incomplete artifact evidence.
