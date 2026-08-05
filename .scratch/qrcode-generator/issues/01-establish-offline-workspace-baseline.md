# 01 — Establish the offline workspace baseline

**What to build:** Convert the initial Leptos scaffold into the planned three-crate workspace with an offline production posture and a repeatable local quality baseline.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] The workspace contains separate core, rendering, and web crates with dependencies flowing only from web to rendering/core and rendering to core.
- [ ] Core and rendering code build and test natively, while the web application builds for WASM through the existing frontend toolchain.
- [ ] Toolchain and dependency versions are deliberately pinned, with browser-only dependencies confined to the web crate and test-only tooling excluded from production dependencies.
- [ ] Documented local commands run formatting, warnings-as-errors linting, native tests, a WASM check, and the production Trunk build.
- [ ] Production HTML and assets make no request for Google Fonts or any other third-party runtime resource.
- [ ] The scaffold continues to build as a usable client-side application without transmitting or logging payload data.
