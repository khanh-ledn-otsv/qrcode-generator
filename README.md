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

```sh
rustup target add wasm32-unknown-unknown
trunk serve --open
```

The root `Trunk.toml` targets `crates/qr-web/index.html`, so Trunk commands run
from the workspace root while the Leptos HTML and styles remain inside `qr-web`.
Trunk builds the Rust application to WebAssembly and compiles Tailwind CSS automatically.

## Verify

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo check --target wasm32-unknown-unknown
trunk build --release
```

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

## Documentation

- [Development plan](docs/DEVELOPMENT_PLAN.md)
- [Testing strategy](docs/TESTING_STRATEGY.md)
