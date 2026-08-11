# Agent Repository Index

## Agent metadata

- **Purpose:** fast repository orientation and entry-point lookup.
- **Read when:** entering the repository or locating a command/document.
- **Authority:** `AGENTS.md` owns execution rules; linked documents own their
  declared technical domains.
- **Do not use for:** deciding which tests to run. Use
  `docs/agents/verification.md`.

## Product boundary

Client-side QR generator: Rust 2024, Leptos, WebAssembly, Trunk, and Tailwind.
Payload processing and artifact generation stay in the browser.

```text
qr-web -> qr-render -> qr-core
   \-----------------> qr-core
```

| Path | Agent meaning |
|---|---|
| `crates/qr-core` | browser-independent encoding |
| `crates/qr-render` | browser-independent deterministic SVG/PNG rendering |
| `crates/qr-web` | Leptos UI and browser integration |
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
builds both. Trunk runs from the repository root; `Trunk.toml` targets
`crates/qr-web/index.html`.

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
|---|---|
| execution constraints and test selection | [`AGENTS.md`](AGENTS.md), [`docs/agents/verification.md`](docs/agents/verification.md) |
| accepted product/architecture decisions | [`docs/DEVELOPMENT_PLAN.md`](docs/DEVELOPMENT_PLAN.md) |
| test-design rationale and required coverage | [`docs/TESTING_STRATEGY.md`](docs/TESTING_STRATEGY.md) |
| specialized release/decoder/coverage commands | [`docs/RELEASE_HARDENING.md`](docs/RELEASE_HARDENING.md) |
| final clean release certification | [`docs/RELEASE_READINESS.md`](docs/RELEASE_READINESS.md) |
| fixture/oracle mutation protocol | [`tests/oracles/README.md`](tests/oracles/README.md) |
| QR source authority/provenance | [`docs/research/qr-public-source-provenance.md`](docs/research/qr-public-source-provenance.md) |
| bundled logo provenance and generated geometry | [`assets/README.md`](assets/README.md), [`docs/generated/logo-placement-policy.md`](docs/generated/logo-placement-policy.md) |
| local issue/domain workflows | [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md), [`docs/agents/domain.md`](docs/agents/domain.md) |
