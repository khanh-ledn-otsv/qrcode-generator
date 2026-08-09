# 26 — Enlarge the exactly centered ONE logo

**What to build:** Display a substantially larger ONE lettermark at the exact center of every supported branded symbol while preserving scan reliability and rejecting geometry that cannot remain centered and function-safe.

**Blocked by:** 24 — Render compact dots with prominent square finders; 25 — Select a branded minimum version in logo mode.

**Status:** resolved

- [x] Each enabled version uses the decode-backed source bounds and knockout clearance selected by Ticket 23, with the unchanged project-owned lettermark embedded at compile time.
- [x] Source artwork and knockout are mathematically centered on both axes; no nearest-safe search or asymmetric shift is permitted.
- [x] The complete knockout intersects only data or remainder modules, stays inside its reviewed version-specific bound, and reports exact obscured-module counts.
- [x] Logo mode continues to require ECC H and an opaque-white background, while transparent output remains available only without the logo.
- [x] Versions whose exact center conflicts with finder, separator, timing, alignment, format, version, or fixed-dark geometry return a typed invalid result with an associated user-facing explanation.
- [x] SVG, PNG, real-size preview, diagnostics, and downloads all use the same accepted logo placement and preserve deterministic output.
- [x] Geometry, structural, pixel, native/WASM, browser, and pinned SVG/PNG decode tests cover every enabled profile/version row and intentional rejection.

## Answer

The renderer now compiles the Ticket 23 placement as its only branded geometry:
Version 6 uses a mathematically centered `13 × 4.875`-module ONE source at
ten-thousandth-module precision and a centered `(13, 17) 15×7` opaque-white
knockout. The unchanged embedded lettermark obscures exactly 105 data modules,
zero remainder modules, and retains six modules of protected clearance.

All other versions return `UnsafeLogoGeometry`; the workflow turns that typed
failure into an associated validation message and disables both downloads.
SVG, direct-RGBA PNG, preview diagnostics, WASM, and browser downloads consume
the same placement. Deterministic hashes were updated, and the pinned
ZXing-C++ reader decodes both PNG and export-density-rasterized SVG artifacts
for every enabled Content, Landing, and Print Version 6 row.
