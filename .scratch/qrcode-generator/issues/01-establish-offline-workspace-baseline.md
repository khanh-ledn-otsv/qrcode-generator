# 01 — Establish the offline workspace baseline

**What to build:** Convert the initial Leptos scaffold into the planned three-crate workspace with an offline production posture and a repeatable local quality baseline.

**Blocked by:** None — can start immediately.

**Status:** resolved

- [x] The workspace contains separate core, rendering, and web crates with dependencies flowing only from web to rendering/core and rendering to core.
- [x] Core and rendering code build and test natively, while the web application builds for WASM through the existing frontend toolchain.
- [x] Toolchain and dependency versions are deliberately pinned, with browser-only dependencies confined to the web crate and test-only tooling excluded from production dependencies.
- [x] Documented local commands run formatting, warnings-as-errors linting, native tests, a WASM check, and the production Trunk build.
- [x] Production HTML and assets make no request for Google Fonts or any other third-party runtime resource.
- [x] The scaffold continues to build as a usable client-side application without transmitting or logging payload data.

## Comments

### Verification — 2026-08-05

The implementation satisfies five of six acceptance criteria. The ticket remains
`ready-for-agent` because the README's verification section does not document an
explicit WASM check such as:

```sh
cargo check --target wasm32-unknown-unknown
```

Verification performed:

- `cargo fmt --check` — passed.
- `cargo check` — passed.
- `cargo test` — passed (the workspace skeleton currently contains no tests).
- `cargo clippy --all-targets --all-features -- -D warnings` — passed.
- `cargo check --target wasm32-unknown-unknown` — passed.
- `NO_COLOR=false trunk build --release` — passed. The literal documented command
  fails in this agent environment before compilation because it inherits
  `NO_COLOR=1`, which Trunk 0.21.14 rejects; this is an environment compatibility
  issue, not a project build failure.

Static inspection confirmed the intended dependency direction, exact direct
dependency/toolchain/tool pins, browser-facing dependencies confined to `qr-web`,
and no third-party runtime resource references in production HTML or CSS. The
generated bootstrap fetches only its local WASM artifact and contains no payload
handling, transmission, or payload logging.

## Answer

Resolved on 2026-08-05. The README now documents the explicit WASM check,
`cargo check --target wasm32-unknown-unknown`, completing the remaining
acceptance criterion.

Final verification passed:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check --target wasm32-unknown-unknown`
- `NO_COLOR=false trunk build --release`
