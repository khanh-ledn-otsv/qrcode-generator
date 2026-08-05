# Repository Instructions

## Scope

These instructions apply to the entire repository.

This is a client-side QR code generator built with Rust 2024, Leptos, WebAssembly, Trunk, and Tailwind CSS. Keep payload processing and artifact generation entirely in the browser.

Before changing architecture, QR encoding, rendering, dependencies, or test policy, read:

- `docs/DEVELOPMENT_PLAN.md`
- `docs/TESTING_STRATEGY.md`

Treat those documents as authoritative. Update them when an implementation decision changes their assumptions.

## Architecture

- Preserve the planned dependency direction: `qr-web -> qr-render -> qr-core`, with `qr-web -> qr-core` also allowed.
- Keep browser APIs out of `qr-core` and `qr-render`.
- Keep encoding separate from rendering; rendering must not alter ECC, version, mask, or encoded modules.
- Treat ISO/IEC 18004:2024 as the normative QR source. Cite the relevant clause or table beside transcribed standard data.
- Use independent QR implementations only as development or test oracles. Do not copy them into production code.

## Implementation Rules

- Preserve user input exactly. Do not trim, normalize, rewrite, log, or transmit QR payloads.
- Do not add production network requests, external fonts, analytics, or telemetry.
- Prefer focused modules, typed errors, checked arithmetic, and bounds-checked access.
- Do not panic on user-controlled input. Avoid `unwrap`, `expect`, and unchecked indexing on user-facing paths.
- Do not add `unsafe` project code without an explicit documented justification.
- Keep generated SVG and PNG output deterministic; exclude timestamps, randomness, unstable ordering, and environment-dependent metadata.
- Preserve semantic HTML, keyboard operation, visible focus, and associated validation messages.
- Keep changes scoped and preserve unrelated work already present in the worktree.

## Dependencies

- Add the smallest dependency needed and separate production dependencies from test-only tooling.
- Pin versions as required by the development plan; do not use wildcards or floating Git revisions.
- Do not add production QR encoders, Reed-Solomon libraries, general image stacks, SVG rasterizers, or scene renderers unless the development plan is deliberately revised.
- Keep browser-only crates and features scoped to `qr-web` after the workspace migration.

## Verification

Run the checks relevant to the changed files before handoff:

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

For changes to the web application, HTML, CSS, WASM boundary, build configuration, or dependencies, also run:

```sh
trunk build --release
```

If required tooling is unavailable, report the skipped check and reason.

## Tests and Fixtures

- Add tests for behavior changes and bug fixes.
- Test public behavior and invariants; do not use the production encoder as its own correctness oracle.
- Cover exact-fit and one-over capacity boundaries, malformed input, deterministic output, and function-module protection when relevant.
- Keep fixtures synthetic and non-sensitive, with recorded provenance.
- Never regenerate golden fixtures implicitly during tests.
- Do not weaken or skip a failing test to make a change pass; fix the cause or document an intentional policy change.

## Handoff

Summarize changed behavior, list verification performed, and identify any checks that could not run.
