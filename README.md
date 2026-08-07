# QR Code Generator

A client-side QR code generator built with Rust, Leptos, WebAssembly, Trunk,
and Tailwind CSS. Payload processing and artifact generation stay in the
browser.

## Workspace

- `qr-core` contains browser-independent QR encoding logic.
- `qr-render` contains browser-independent deterministic artifact rendering.
- `qr-web` contains the Leptos application and browser integrations.
- `fixture-tool` is a development-only manifest, golden-diff, and independent
  decoder harness. It is not a dependency of any production crate.

Dependencies flow from `qr-web` to `qr-render` and `qr-core`, and from
`qr-render` to `qr-core`.

## Run locally

Use Node.js v24 (declared in `.nvmrc`), pnpm 11.20.0, and `uv` for the
development-only Python environment.

```sh
./scripts/setup.sh
pnpm run dev
```

The root `Trunk.toml` targets `crates/qr-web/index.html`, so Trunk commands run
from the workspace root while the Leptos HTML and styles remain inside `qr-web`.
Trunk builds the Rust application to WebAssembly and compiles Tailwind CSS automatically.

## Verify

Run the complete repository gate with one command:

```sh
pnpm run verify
```

Use `pnpm run check` for static checks and the release build, `pnpm run test`
for all native, Python, WASM, and browser tests, and `pnpm run format` to apply
Rust, TypeScript, and Python formatters.

Committed QR fixtures are verified without regeneration during `cargo test`.
The explicit dual-oracle generation and ZXing-C++ decode workflow is documented
in [`tests/oracles/README.md`](tests/oracles/README.md).

Replay the committed core robustness regressions and run the payload-silent
native diagnostic example with:

```sh
cargo test -p qr-core --test fuzz_regressions
cargo fuzz run encode -- -runs=10000
printf %s 'synthetic diagnostic input' | cargo run -p qr-core --example diagnostics
```

Extended coverage, mutation, fuzz, Miri, dependency, performance, adverse-image,
compressed-bundle, and release-evidence commands are documented in
[`docs/RELEASE_HARDENING.md`](docs/RELEASE_HARDENING.md).

## Documentation

- [Development plan](docs/DEVELOPMENT_PLAN.md)
- [Testing strategy](docs/TESTING_STRATEGY.md)
- [Release hardening](docs/RELEASE_HARDENING.md)
