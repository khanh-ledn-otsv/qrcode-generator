# 02 — Define validated output profiles and canvas geometry

**What to build:** Provide four compiled output profiles whose QR version ceilings and fixed-canvas geometry can be validated independently of encoding and browser behavior.

**Blocked by:** 01 — Establish the offline workspace baseline.

**Status:** resolved

- [x] Each supported profile has typed base dimensions, PNG dimensions exactly three times the base dimensions, and an explicit maximum QR version.
- [x] Geometry includes four quiet modules per side and chooses the largest positive even integer module scale that fits the canvas.
- [x] Outer padding is checked, symmetric, integral, and contains background only.
- [x] Every version allowed by every profile is exercised, including scale transitions and the maximum-version minimum of six pixels per module.
- [x] Invalid profiles, impossible dimensions, and arithmetic overflow return typed errors without panic.

## Answer

Resolved on 2026-08-05. `qr-core` now owns the validated QR version type,
while `qr-render` provides the four compiled profiles and checked fixed-canvas
geometry. The public geometry records the four-module quiet zone, largest
fitting positive even scale, rendered bounds, symmetric integral padding, and
the padding's background-only content rule.

Exhaustive tests cover all 38 profile/version combinations and their scale
transitions, with worked ceiling examples and property coverage for geometry
invariants and malformed profiles. Typed errors cover invalid profile data,
impossible dimensions, asymmetric padding, version ceilings, and overflow.

Verification passed:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check --target wasm32-unknown-unknown`
- `NO_COLOR=false trunk build --release`
