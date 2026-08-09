# Release hardening

This runbook turns the approved output surface into reproducible local release
evidence. Run it from a clean checkout with Node.js 24 from `.nvmrc`, pnpm
11.20.0, Rust 1.97.1, and the locked oracle environment.

## Tool setup

Run `./scripts/setup.sh` first. The extended gates additionally require these
pinned tools:

```sh
rustup component add llvm-tools-preview
cargo install --locked cargo-llvm-cov --version 0.8.6
cargo install --locked cargo-mutants --version 27.1.0
cargo install --locked cargo-audit --version 0.22.2
cargo install --locked cargo-fuzz --version 0.13.2
rustup toolchain install nightly --component miri
```

The ZXing-C++ checkout and executable are pinned by
`tests/fixtures/manifest.json` and built by `./scripts/setup.sh`. Release
commands fail when that independent decoder is absent or at the wrong commit.
Artifact decoding uses ZXing's explicit fixed binarizer. Its default local
binarizer fails a perfectly uniform 360 px Version 3 artifact while the same
matrix decodes at the other approved sizes and with ZXing fixed and pure-symbol
modes. The global binarizer also rejects the dense anti-aliased print SVG.
Pinning fixed thresholding removes both image-dependent oracle preprocessing
ambiguities while preserving detection and decode checks.

Independent SVG decoding rasterizes the vector at the profile's 3× export
density. Structural tests separately enforce the required base `width` and
`height`. The dense Version 13 vector has only about two pixels per module when
rasterized at its base CSS size and no ZXing binarizer decodes that low-density
raster; at export density the same resolution-independent artifact decodes.

## Routine and extended local gates

The complete routine gate remains:

```sh
pnpm run verify
```

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
98/95, and plain-Rust web workflow state 85/80. Mutation scoring excludes
unviable mutants, fails on untriaged timeouts, and enforces 85% for qr-core and
90% for both critical core files and profile geometry.

## Artifact evidence

Run:

```sh
pnpm run release:evidence
```

This writes the approved matrix, adverse decoder outcomes, and artifact SHA-256
hashes under `target/release-evidence/`. The matrix has 248 generated scenario
rows: 96 required-payload rows and 152 exact-version coverage rows spanning all
compiled profile, background, logo, payload, and enabled-version paths. Each row
contains matched native-PNG and independently rasterized SVG evidence. The 142
renderable rows record safety, deterministic artifact and decoder-input hashes,
a ZXing decode, and reviewed logo geometry where applicable; the 106 unsupported
logo/background or centered-logo geometry rows record the expected typed
rejection.
`tests/adverse/parameters.json` is the versioned transform manifest. The adverse
evidence records exactly 29 decoded outcomes across three declared envelopes:
all 13 transforms for a low-density opaque Print compact-dot symbol (`safe`),
10 placement-relevant transforms for transparent compact dots (`caution`), and
six transforms for the centered Version 6 Print logo (`caution`). It compares
decoded pixels before invoking ZXing and includes light, darker, and patterned
placement backgrounds; each evidence row records the configuration, safety
class, transform, decoder, and outcome.

Approved matrix artifact and allocation ceilings live in
`tests/baselines/resources.json` and are exercised in ordinary native tests.

Use [`RELEASE_READINESS.md`](RELEASE_READINESS.md) for the final clean-build,
browser, artifact, and privacy gate. Manual product checks remain outside the
repository evidence system.
