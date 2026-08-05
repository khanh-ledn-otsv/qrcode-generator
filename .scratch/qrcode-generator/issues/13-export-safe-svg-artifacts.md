# 13 — Export deterministic safe SVG artifacts

**What to build:** Export the safe render model as a compact, secure, deterministic SVG that preserves exact symbol geometry and independently decodes.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 12 — Create the safe render model and fixed-canvas placement.

**Status:** ready-for-agent

- [ ] SVG dimensions and viewBox exactly cover the QR symbol including its quiet zone, while profile scaling remains available to consumers.
- [ ] Module paths use stable ordering and numeric formatting and remain inside their assigned cells.
- [ ] Output contains no scripts, events, remote URLs, external stylesheets, foreign objects, payload metadata, borders, labels, or strokes.
- [ ] Background treatment and quiet-zone behavior are structurally verified by parsing the generated artifact.
- [ ] Repeated requests produce byte-identical UTF-8 output.
- [ ] Pinned independent rasterization followed by independent decoding recovers the exact original payload across representative profiles and versions.
