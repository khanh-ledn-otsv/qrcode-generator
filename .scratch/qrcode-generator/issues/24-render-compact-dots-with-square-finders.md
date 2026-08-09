# 24 — Render compact dots with prominent square finders

**What to build:** Make generated SVG, PNG, previews, and downloads use the approved compact-dot field while the three corner finder patterns remain large, solid, and square like the reference.

**Blocked by:** 23 — Approve decode-backed branded geometry.

**Status:** resolved

- [x] Production uses one compiled branded appearance rather than exposing arbitrary radii, per-module callbacks, rounded-style compatibility, or a new shape selector.
- [x] Every approved dot is centered in its original module cell, stays within that cell, and uses the exact decode-backed diameter selected by Ticket 23.
- [x] All three 7×7 finder regions remain full-size square patterns, separators remain blank, and every protected pattern follows the approved Ticket 23 treatment.
- [x] SVG emits stable row-major square and dot geometry with fixed numeric formatting, exact dimensions, and no decorative outer frame inside the exported artifact.
- [x] PNG rasterizes dot coverage deterministically on opaque and transparent backgrounds while confining intermediate edge colors or alpha to the mathematically approved dot envelope.
- [x] The four-module quiet zone, fixed-canvas background-only padding, exact preview size, deterministic downloads, and payload privacy remain unchanged.
- [x] Structural, pixel, native/WASM determinism, browser workflow, and pinned independent-decoder tests cover the branded dot output.

## Answer

Production SVG, PNG, previews, and downloads now use one compiled 0.45-module
centered dot for every visible non-finder module. Finder modules remain exact
full-cell squares and separator modules remain blank through the shared
row-major glyph ownership model.

SVG emits deterministic mixed square/circular path commands at fixed
thousandth-module precision. PNG uses deterministic 8×8 subpixel coverage;
opaque edges blend only against the approved background and transparent edges
retain brand RGB with coverage alpha. Structural and pixel tests cover the
approved envelope, all supported profile/version rows, exact artifact hashes,
and the native/WASM PNG fixture. The browser diagnostic now reports “Compact
dots” while retaining the standard-square finder treatment and no shape
control.
