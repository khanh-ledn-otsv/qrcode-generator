# 14 — Export deterministic safe PNG artifacts

**What to build:** Export the safe render model through a direct RGBA buffer and deterministic PNG serialization, without browser rasterization or a production image stack.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 12 — Create the safe render model and fixed-canvas placement.

**Status:** ready-for-agent

- [ ] Square modules are filled as exact integer pixel rectangles in a checked RGBA buffer at the profile’s PNG dimensions.
- [ ] PNG settings explicitly fix color type, bit depth, filter, compression, chunk order, and metadata policy.
- [ ] Artifacts contain valid PNG structure with no timestamps or payload-bearing text chunks.
- [ ] Quiet zones and surplus outer padding contain exactly the selected background treatment and no artwork.
- [ ] Repeated native and WASM requests are byte-identical where cross-target output is specified to match.
- [ ] Parsed artifact pixels independently decode to the exact original payload across representative profiles and versions.
