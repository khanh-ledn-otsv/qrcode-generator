# 13 — Export deterministic safe SVG artifacts

**What to build:** Export the safe render model as a compact, secure, deterministic SVG that preserves exact symbol geometry and independently decodes.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 12 — Create the safe render model and fixed-canvas placement.

**Status:** ready-for-agent

- [ ] SVG `width` and `height` equal the selected profile's base dimensions.
- [ ] SVG `viewBox` is exactly `0 0 N N`, where `N` is the matrix width plus eight modules for exactly four quiet-zone modules per side; no fixed-canvas surplus padding appears in the viewBox.
- [ ] Module paths use stable ordering and numeric formatting and remain inside their assigned cells.
- [ ] Output contains no scripts, events, remote URLs, external stylesheets, foreign objects, payload metadata, borders, labels, or strokes.
- [ ] Background treatment and quiet-zone behavior are structurally verified by parsing the generated artifact.
- [ ] Repeated requests produce byte-identical UTF-8 output.
- [ ] Pinned independent rasterization followed by independent decoding recovers the exact original payload across representative profiles and versions.
