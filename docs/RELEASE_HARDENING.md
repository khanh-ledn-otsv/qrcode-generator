# Release hardening

## Agent metadata

- **Purpose:** command/evidence contract for specialized hardening suites.
- **Read when:** a specialized trigger in `AGENTS.md` applies or the user asks
  for release/decoder/coverage/mutation/fuzz/Miri evidence.
- **Authority:** specialized command semantics and evidence outputs.
- **Default:** do not run this runbook end to end. Select one triggered row;
  final certification uses `RELEASE_READINESS.md`.
- **Cost warning:** decoder/evidence suites take minutes; mutation, fuzz, and
  Miri are campaigns. Tool installation is required only for the selected row.

This runbook turns the approved output surface into reproducible local release
evidence. Use Node.js 24 from `.nvmrc`, pnpm 11.20.0, Rust 1.98.0, and the
locked oracle environment. Require a clean worktree only when the selected
command says so; never alter user changes to create one.

## Tool setup

Run `./scripts/setup.sh` only when the selected gate needs decoder/tool setup.
Install only the pinned tool required by the selected row:

```sh
rustup component add llvm-tools-preview
cargo install --locked cargo-llvm-cov --version 0.8.6
cargo install --locked cargo-mutants --version 27.1.0
cargo install --locked cargo-audit --version 0.22.2
cargo install --locked cargo-fuzz --version 0.13.2
rustup toolchain install nightly --component miri
```

The ZXing-C++ and quirc checkouts and executables are pinned by
`tests/fixtures/manifest.json` and built by `./scripts/setup.sh`. Release
commands fail when an independent decoder is absent, modified, or at the wrong
commit. The representative secondary ASCII raster suite runs with
`pnpm run test:quirc`; ZXing-C++ remains authoritative for UTF-8/ECI metadata.
Artifact decoding uses ZXing's explicit fixed binarizer. Its default local
binarizer fails a perfectly uniform 360 px Version 3 artifact while the same
matrix decodes at the other approved sizes and with ZXing fixed and pure-symbol
modes. The global binarizer also rejects the dense anti-aliased print SVG.
Pinning fixed thresholding removes both image-dependent oracle preprocessing
ambiguities while preserving detection and decode checks.

Independent SVG decoding rasterizes the vector at the profile's PNG export
dimensions, exactly 3× the SVG dimensions for every fixed profile. Structural
tests separately enforce the required base `width` and `height`.

## Routine and extended local gates

Choose the routine gate first via [`agents/verification.md`](agents/verification.md).
The following rows are specialized additions, never a default checklist.

The following commands expose the hardening seams individually:

| Evidence | Command |
|---|---|
| QR tables and invariants | `cargo test -p qr-core --test tables` |
| Golden matrices and oracle policy | `cargo test -p fixture-tool && pnpm run test:python` |
| Approved tuple table and resource ceilings | `pnpm run test:approved` |
| Chromium/WASM matrix | `pnpm run test:wasm && pnpm run test:e2e` |
| Independent SVG/PNG decoding | `pnpm run test:decode` |
| Deterministic adverse envelope | `pnpm run test:adverse:decode` |
| Coverage thresholds | `pnpm run release:coverage` |
| Mutation thresholds | `pnpm run release:mutation` |
| Ten-minute-per-target extended fuzz budget | `pnpm run release:fuzz` |
| One-hour critical-target fuzz budget | `pnpm run release:fuzz:deep` |
| Miri core/geometry checks | `pnpm run release:miri` |
| Dependency advisories/duplicates | `pnpm run release:dependencies` |

Coverage is checked without filename exclusions. The enforced scopes and
line/region minima are the ones in `TESTING_STRATEGY.md`: qr-core 95/90,
critical arithmetic/matrix files 98/95, qr-render 90/85, geometry/profile
98/95, and the private web generation policy 85/80. Mutation scoring excludes
unviable mutants, fails on untriaged timeouts, and enforces 85% for qr-core and
90% for both critical core files and profile geometry.

## Artifact evidence

Run:

```sh
pnpm run release:evidence
```

This writes the approved matrix, adverse decoder outcomes, and artifact SHA-256
hashes under `target/release-evidence/`. The matrix has 436 generated scenario
rows: 120 required-payload rows and 316 exact-version coverage rows spanning all
compiled profile, logo, foreground-theme, payload, and enabled-version paths. Each row
contains matched native-PNG and independently rasterized SVG evidence. The 290
renderable rows record safety, deterministic artifact and decoder-input hashes,
a ZXing decode, foreground theme, and reviewed fixed/adaptive logo geometry where applicable; the 146 unsupported
logo or profile-specific geometry rows record the expected typed
rejection.
The executable counts and dimensions have one versioned owner in
`tests/approved-output-matrix-policy.json`; the Rust coverage test and Python
readiness validator both reject drift from it.
`tests/adverse/parameters.json` is the versioned transform manifest. The adverse
evidence records exactly 29 decoded outcomes across four declared envelopes:
all 13 transforms for a low-density opaque Print Rounded ONE symbol (`safe`),
six transforms for the centered Version 6 Print logo (`caution`), and five each
for Adaptive Version 10 and Version 11 long-URL artifacts (`caution`). It compares
decoded pixels before invoking ZXing and includes light, darker, and patterned
placement backgrounds; each evidence row records the configuration, safety
class, transform, decoder, and outcome.

Adaptive release evidence additionally covers selected-version dimensions
through Version 40, the exact Version 40 ECC-M byte boundary, decode-approved
branding through Version 11, and typed branded rejection from Version 12
through Version 40. The Version 40 SVG/PNG hashes are shared by native and WASM
tests. Chromium repeats both downloads and independently decodes the PNG; the
all-version rasterized-SVG campaign supplies the SVG decode evidence. A
separate request-interception workflow generates and downloads the Version 40
PNG without a runtime network request. Recorded artifact ceilings are
1,070,097 SVG bytes, 73,762 PNG bytes, and 4,928,400 bytes for the direct RGBA
buffer.

Approved matrix artifact and allocation ceilings live in
`tests/baselines/resources.json`. Ordinary native tests exercise every
renderable profile/logo tuple plus the largest Adaptive Version 40 boundary;
release evidence and extended CI exercise every approved matrix row against
the same ceilings.

Use [`RELEASE_READINESS.md`](RELEASE_READINESS.md) for the final clean-build,
browser, artifact, and privacy gate. Manual product checks remain outside the
repository evidence system.
