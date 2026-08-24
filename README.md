# QR Code Generator

A privacy-first QR Code generator built with Astro, Tailwind CSS, Rust, and WebAssembly.
Payload processing and artifact generation happen in the browser, so the QR
content stays on the user's device. The project is designed for people who
need a QR generator they can inspect, test, and trust rather than a black-box
online service.

## Why trust this project?

### Rust safe mode

The production crates are written in Rust 2024 and explicitly use
`#![forbid(unsafe_code)]` in `qr-core`, `qr-render`, and `qr-web`. The same rule
also applies to the fixture tooling. This means project code cannot opt into
Rust's `unsafe` operations without the compiler rejecting it.

That compiler guarantee is reinforced by checked arithmetic, bounds-checked
matrix access, typed errors for invalid input and unsafe logo geometry, and
defensive resource ceilings. Dependencies are still audited separately; the
project's `unsafe_code` prohibition applies to project source, not third-party
crates.

### Five layers of tests

Every applicable layer tests a different failure mode. A successful scan alone
is not treated as proof of QR correctness.

1. **Conformance:** bit encoding, error correction, function patterns, masks,
   version information, and all 160 version/ECC table rows are checked against
   committed, independently corroborated fixtures.
2. **Structural invariants:** matrix ownership, quiet zones, dimensions,
   checked capacities, logo placement, and deterministic artifact structure are
   validated directly.
3. **Independent decoding:** generated PNGs and rasterized SVGs are decoded by
   pinned ZXing-C++, with quirc used as a second decoder for representative
   ASCII cases. These tools are test oracles only and are not production
   dependencies.
4. **Browser behavior:** native Rust, WebAssembly, and desktop Chromium tests
   cover the actual browser boundary, downloads, keyboard interaction, privacy
   behavior, and independent PNG decoding.
5. **Robustness:** property tests, adverse-image transformations, mutation
   testing, coverage checks, fuzz targets, and selected Miri runs exercise edge
   cases beyond hand-written examples.

The release evidence matrix contains 436 approved scenarios, including 290
renderable artifact rows and 146 intentional typed-rejection rows. The
hardening suite also records 29 deterministic adverse-image outcomes. Critical
coverage targets are enforced at 95% line / 90% region coverage for `qr-core`,
98% / 95% for critical arithmetic and matrix code, 90% / 85% for `qr-render`,
and 98% / 95% for profile geometry.

### Conservative output defaults

The generator uses opaque white backgrounds, approved foreground colors, a
4.5:1 minimum contrast ratio, and a four-module quiet zone. Logo mode raises
error correction to H and is enabled only for decode-tested geometry. If a
logo placement would overlap QR function modules or exceed an approved
capacity, the unsafe branded export is rejected or falls back to the same
exact-payload QR without a logo.

## Verification

The routine repository gate covers formatting, Rust checks and tests, native
and WebAssembly tests, Python oracle checks, an optimized build, and Chromium
tests:

```sh
pnpm run verify
```

For release work, the repository also provides independent decoder evidence,
coverage, mutation, fuzzing, Miri, dependency-audit, and release-readiness
commands. See [`docs/TESTING_STRATEGY.md`](docs/TESTING_STRATEGY.md) and
[`docs/RELEASE_HARDENING.md`](docs/RELEASE_HARDENING.md) for the scope and
provenance of each check.

## Agent metadata

- **Purpose:** fast repository orientation and entry-point lookup.
- **Read when:** entering the repository or locating a command/document.
- **Authority:** `AGENTS.md` owns execution rules; linked documents own their
  declared technical domains.
- **Do not use for:** deciding which tests to run. Use
  `docs/agents/verification.md`.

## Product boundary

Client-side QR generator: Astro, Tailwind CSS, Rust 2024, and WebAssembly.
Payload processing and artifact generation stay in the browser.

```text
qr-web -> qr-render -> qr-core
   \-----------------> qr-core
```

| Path | Agent meaning |
| --- | --- |
| `crates/qr-core` | browser-independent encoding |
| `crates/qr-render` | browser-independent deterministic SVG/PNG rendering |
| `crates/qr-web` | Rust WASM workflow and browser download adapter |
| `crates/fixture-tool` | development-only fixture/diff/decoder harness |
| `tests/oracles` | locked Python oracle environment |
| `scripts` | repository command implementations and orchestration |

## Environment and entry points

- Node: `.nvmrc` (`v24`); verify before any Node-backed command.
- Package manager: exact `packageManager` value in `package.json`.
- Rust: `rust-toolchain.toml`.
- Python: locked uv project under `tests/oracles`.
- Local sccache: optional `RUSTC_WRAPPER=sccache`, version `0.17.0`.
- Hosted sccache integration: `mozilla-actions/sccache-action@v0.0.11`, using
  the pinned compiler-cache version above.

```sh
./scripts/setup.sh
pnpm run dev
```

`setup.sh` accepts `QR_DECODER_SETUP_MODE=zxing` or
`QR_DECODER_SETUP_MODE=quirc` to avoid building an unused oracle. The default
builds the Rust WASM adapter and Astro application.

For verification, run exactly the gate selected by `AGENTS.md` and
[`docs/agents/verification.md`](docs/agents/verification.md). Do not infer a
test plan from the number of scripts in `package.json`.

## Hosted flow

`Correctness` selects a covering gate for each relevant push. On an eligible
`main` push it publishes the already verified release artifact to Pages with
the configured base path; it does not repeat the Rust release build. Extended
decoder CI is separately path-filtered. Both workflows support manual dispatch.

## Document retrieval map

| Need | Read |
| --- | --- |
| execution constraints and test selection | [`AGENTS.md`](AGENTS.md), [`docs/agents/verification.md`](docs/agents/verification.md) |
| accepted product/architecture decisions | [`docs/DEVELOPMENT_PLAN.md`](docs/DEVELOPMENT_PLAN.md) |
| test-design rationale and required coverage | [`docs/TESTING_STRATEGY.md`](docs/TESTING_STRATEGY.md) |
| specialized release/decoder/coverage commands | [`docs/RELEASE_HARDENING.md`](docs/RELEASE_HARDENING.md) |
| final clean release certification | [`docs/RELEASE_READINESS.md`](docs/RELEASE_READINESS.md) |
| fixture/oracle mutation protocol | [`tests/oracles/README.md`](tests/oracles/README.md) |
| QR source authority/provenance | [`docs/research/qr-public-source-provenance.md`](docs/research/qr-public-source-provenance.md) |
| bundled logo provenance and generated geometry | [`assets/README.md`](assets/README.md), [`docs/generated/logo-placement-policy.md`](docs/generated/logo-placement-policy.md) |
| local issue/domain workflows | [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md), [`docs/agents/domain.md`](docs/agents/domain.md) |
