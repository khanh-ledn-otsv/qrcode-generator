# QR Code Generator

A client-side QR code generator built with Rust, Leptos, WebAssembly, Trunk,
and Tailwind CSS. Payload processing and artifact generation stay in the
browser.

## Workspace

- `qr-core` contains browser-independent QR encoding logic.
- `qr-render` contains browser-independent deterministic artifact rendering.
- `qr-web` contains the Leptos application and browser integrations.

Dependencies flow from `qr-web` to `qr-render` and `qr-core`, and from
`qr-render` to `qr-core`.

## Run locally

```sh
rustup target add wasm32-unknown-unknown
trunk serve --open
```

Trunk builds the Rust application to WebAssembly and compiles Tailwind CSS automatically.

## Verify

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
trunk build --release
```

## Documentation

- [Development plan](docs/DEVELOPMENT_PLAN.md)
- [Testing strategy](docs/TESTING_STRATEGY.md)
