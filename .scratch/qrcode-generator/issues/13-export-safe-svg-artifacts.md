# 13 — Export deterministic safe SVG artifacts

**What to build:** Export the safe render model as a compact, secure, deterministic SVG that preserves exact symbol geometry and independently decodes.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 12 — Create the safe render model and fixed-canvas placement.

**Status:** resolved

- [x] SVG `width` and `height` equal the selected profile's base dimensions.
- [x] SVG `viewBox` is exactly `0 0 N N`, where `N` is the matrix width plus eight modules for exactly four quiet-zone modules per side; no fixed-canvas surplus padding appears in the viewBox.
- [x] Module paths use stable ordering and numeric formatting and remain inside their assigned cells.
- [x] Output contains no scripts, events, remote URLs, external stylesheets, foreign objects, payload metadata, borders, labels, or strokes.
- [x] Background treatment and quiet-zone behavior are structurally verified by parsing the generated artifact.
- [x] Repeated requests produce byte-identical UTF-8 output.
- [x] Pinned independent rasterization followed by independent decoding recovers the exact original payload across representative profiles and versions.

## Implementation summary

Added a dependency-free production `render_svg(&RenderModel) -> Result<String,
RenderError>` boundary. It emits exact profile dimensions, a tight logical
view box, one opaque background rectangle, and one stable row-major path of
integer square-module subpaths. Payload text and all active/external content,
metadata, borders, labels, stylesheets, and strokes are absent by construction.

Pinned test-only `roxmltree` 0.21.1 and `resvg` 0.48.1 validate parsed artifact
structure and independently rasterized pixels. Structural coverage exercises
every version allowed by every profile, while the pinned ZXing-C++ 3.0.2
integration decodes exact bytes and metadata for all 38 allowed profile/version
tuples. A fixed SHA-256 artifact hash is the serialization determinism gate.
The real decoder run also corrected the shared fixture boundary to expect
ZXing's pinned `QR Code` format label.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, Clippy
for all targets/features with warnings denied, `cargo check -p qr-render
--target wasm32-unknown-unknown`, the explicit pinned ZXing SVG decode suite,
and `trunk build --release` with `NO_COLOR=true` for Trunk 0.21.14.
