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
cargo install --locked cargo-bloat --version 0.12.1
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
| Browser/WASM matrix | `pnpm run test:wasm && pnpm run test:e2e:all` |
| Independent SVG/PNG decoding | `pnpm run test:decode` |
| Deterministic adverse envelope | `pnpm run test:adverse:decode` |
| Coverage thresholds | `pnpm run release:coverage` |
| Mutation thresholds | `pnpm run release:mutation` |
| Ten-minute-per-target extended fuzz budget | `pnpm run release:fuzz` |
| One-hour critical-target fuzz budget | `pnpm run release:fuzz:deep` |
| Miri core/geometry checks | `pnpm run release:miri` |
| Dependency advisories/duplicates | `pnpm run release:dependencies` |
| Criterion performance distributions | `pnpm run release:performance -- --save-baseline release` |
| Compressed WASM ceiling | `pnpm run release:bundle` |
| WASM size attribution | `pnpm run release:size-attribution` |

Coverage is checked without filename exclusions. The enforced scopes and
line/region minima are the ones in `TESTING_STRATEGY.md`: qr-core 95/90,
critical arithmetic/matrix files 98/95, qr-render 90/85, geometry/profile
98/95, and plain-Rust web workflow state 85/80. Mutation scoring excludes
unviable mutants, fails on untriaged timeouts, and enforces 85% for qr-core and
90% for both critical core files and profile geometry.

Criterion writes median and distribution estimates under `target/criterion`.
Use `--baseline release` on the next stable-machine run to obtain statistical
change reports; correctness tests never assert elapsed milliseconds.

## Artifact evidence

Run:

```sh
pnpm run release:evidence
```

This writes the approved matrix, adverse decoder outcomes, deterministic gzip
measurement, and artifact SHA-256 hashes under `target/release-evidence/`.
Each of the 192 generated matrix rows identifies all seven configuration
dimensions plus its payload class. The 144 renderable rows record safety and a
ZXing decode; the 48 bundled-logo/transparent rows record the expected typed
rejection. `tests/adverse/parameters.json` is the versioned transform manifest.
The adverse evidence applies all 13 named transforms to the safe square
baseline, a 10-transform envelope to rounded transparent output, and a
six-transform envelope to logo output. It compares decoded pixels before
invoking ZXing and includes light, darker, and patterned placement backgrounds;
each evidence row records the configuration, safety class, transform, decoder,
and outcome.

The optimized WASM baseline measured on 2026-08-07 is 154,450 gzip bytes. The
160,000-byte ceiling allows 3.6% headroom and fails larger regressions. Approved
matrix artifact and allocation ceilings live in
`tests/baselines/resources.json` and are exercised in ordinary native tests.

Release evidence is incomplete until the owner also records the named physical
device, scanner, printer, stock/material, 25 mm and 30 mm print, and placement
matrix required by `TESTING_STRATEGY.md`. Those manual results are deliberately
not fabricated by repository automation.

Use [`RELEASE_READINESS.md`](RELEASE_READINESS.md) for the final clean-build,
browser, manual-evidence, exception, and sign-off gate.
