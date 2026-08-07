# 26 — Enlarge the exactly centered ONE logo

**What to build:** Display a substantially larger ONE lettermark at the exact center of every supported branded symbol while preserving scan reliability and rejecting geometry that cannot remain centered and function-safe.

**Blocked by:** 24 — Render compact dots with prominent square finders; 25 — Select a branded minimum version in logo mode.

**Status:** ready-for-agent

- [ ] Each enabled version uses the decode-backed source bounds and knockout clearance selected by Ticket 23, with the unchanged project-owned lettermark embedded at compile time.
- [ ] Source artwork and knockout are mathematically centered on both axes; no nearest-safe search or asymmetric shift is permitted.
- [ ] The complete knockout intersects only data or remainder modules, stays inside its reviewed version-specific bound, and reports exact obscured-module counts.
- [ ] Logo mode continues to require ECC H and an opaque-white background, while transparent output remains available only without the logo.
- [ ] Versions whose exact center conflicts with finder, separator, timing, alignment, format, version, or fixed-dark geometry return a typed invalid result with an associated user-facing explanation.
- [ ] SVG, PNG, real-size preview, diagnostics, and downloads all use the same accepted logo placement and preserve deterministic output.
- [ ] Geometry, structural, pixel, native/WASM, browser, and pinned SVG/PNG decode tests cover every enabled profile/version row and intentional rejection.

