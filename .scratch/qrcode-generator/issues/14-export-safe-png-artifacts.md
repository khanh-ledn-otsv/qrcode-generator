# 14 — Export deterministic safe PNG artifacts

**What to build:** Export the safe render model through a direct RGBA buffer and deterministic PNG serialization, without browser rasterization or a production image stack.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 12 — Create the safe render model and fixed-canvas placement.

**Status:** resolved

- [x] Square modules are filled as exact integer pixel rectangles in a checked RGBA buffer at the profile’s PNG dimensions.
- [x] PNG settings explicitly fix color type, bit depth, filter, compression, chunk order, and metadata policy.
- [x] Artifacts contain valid PNG structure with no timestamps or payload-bearing text chunks.
- [x] Quiet zones and surplus outer padding contain exactly the selected background treatment and no artwork.
- [x] Repeated native and WASM requests are byte-identical where cross-target output is specified to match.
- [x] Parsed artifact pixels independently decode to the exact original payload across representative profiles and versions.

## Answer

Resolved on 2026-08-06. `qr-render` now exposes `render_png`, which fills the
prevalidated profile-sized RGBA buffer with the selected background and paints
dark safe-preset modules as checked, exact integer rectangles. PNG 0.18.1 is a
pinned production dependency configured explicitly for RGBA, 8-bit depth,
balanced compression, no scanline filter, a single validated image sequence,
and no metadata chunks.

Artifact tests parse the PNG structure and decoded pixels for every supported
profile/version, enforce the `IHDR`/`IDAT`/`IEND`-only chunk policy, and freeze a
SHA-256 artifact contract. The same hash passed natively and in a Node-backed
`wasm32-unknown-unknown` test. The pinned ZXing-C++ 3.0.2 reader independently
decoded the emitted PNGs to the exact original payload across every supported
profile/version combination.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, Clippy
for all targets/features with warnings denied, `cargo check --target
wasm32-unknown-unknown`, `NO_COLOR=true trunk build --release`, the dedicated
WASM determinism test, and the ignored-by-default independent PNG decode suite.
The full native suite required process-local `commit.gpgsign=false` because its
temporary repositories otherwise inherited unavailable 1Password signing.
