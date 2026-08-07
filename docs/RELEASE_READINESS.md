# Release readiness

This runbook is the final release gate. It does not infer or manufacture manual
results: the release owner must name the supported environments and sign the
evidence for the exact commit being released.

## Automated evidence

From a clean worktree, run setup and the readiness gate:

```sh
./scripts/setup.sh
cp tests/release/manual-evidence.template.json tests/release/manual-evidence.json
# Replace every placeholder and attach real evidence before continuing.
pnpm run release:readiness
```

The command verifies pinned tool versions, produces two release builds in
separate Cargo target directories, compares every application artifact by
SHA-256, records deterministic gzip size for the WASM artifact, runs Chromium,
mobile Chromium, Firefox, and WebKit with Playwright retries disabled, and runs
the approved-output decoder evidence. It writes the machine evidence and final
report under `target/release-readiness/`.

`tests/release/manual-evidence.json` is intentionally untracked. Preserve the
signed copy and referenced attachments with the release record. To store it at
another location, set `QR_MANUAL_EVIDENCE` to that file before running the gate.

## Manual evidence

Replace every template value with a precise product and version, device model,
scanner application, screen, printer, material/stock, and placement environment.
Map each supported browser to the Playwright project that supplies its automated
evidence; this mapping is an evidence label, not a claim that emulation replaces
testing on the named browser and device.
Test actual 25 mm and 30 mm print samples and link their retained evidence.
Record at least one VoiceOver/browser pair and one NVDA/browser pair. The release
candidate must be the full commit hash emitted by `git rev-parse HEAD`.

If a physical test cannot be completed, the only substitute is an exception in
the manual evidence with criterion `physical-validation`, a concrete reason,
signer, and ISO date. An unsigned omission never passes.

## Acceptance-criterion map

| Criterion | Evidence | Gate |
|---|---|---|
| No runtime payload or logo requests | Playwright privacy test in every configured project | `automated.network_inspection` |
| Clean pinned reproducible build and compressed WASM | tool capture plus two artifact hash maps | `automated.reproducible_builds`, `automated.compressed_wasm` |
| Desktop/mobile critical paths, downloads, privacy, accessibility, zero retries | all four Playwright projects with `retries: 0` | `automated.browsers`, `automated.downloads`, `automated.accessibility` |
| Named cameras, scanners, screens, printers, materials, placements, 25/30 mm prints | owner evidence or signed exception | `manual.physical_results`, `manual.exceptions` |
| SVG-first, sizing, transparent/logo, environment guidance | browser assertion against visible guidance | `automated.guidance` |
| Complete criterion mapping and sign-off | validated report for the same commit | `target/release-readiness/readiness-report.json` |

The validator rejects placeholders, missing categories, failed or absent print
sizes, retry-based browser results, mismatched commits, missing accessibility
pairs, and unsigned release decisions.
