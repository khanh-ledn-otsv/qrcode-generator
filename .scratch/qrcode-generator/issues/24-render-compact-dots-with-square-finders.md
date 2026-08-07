# 24 — Render compact dots with prominent square finders

**What to build:** Make generated SVG, PNG, previews, and downloads use the approved compact-dot field while the three corner finder patterns remain large, solid, and square like the reference.

**Blocked by:** 23 — Approve decode-backed branded geometry.

**Status:** ready-for-agent

- [ ] Production uses one compiled branded appearance rather than exposing arbitrary radii, per-module callbacks, rounded-style compatibility, or a new shape selector.
- [ ] Every approved dot is centered in its original module cell, stays within that cell, and uses the exact decode-backed diameter selected by Ticket 23.
- [ ] All three 7×7 finder regions remain full-size square patterns, separators remain blank, and every protected pattern follows the approved Ticket 23 treatment.
- [ ] SVG emits stable row-major square and dot geometry with fixed numeric formatting, exact dimensions, and no decorative outer frame inside the exported artifact.
- [ ] PNG rasterizes dot coverage deterministically on opaque and transparent backgrounds while confining intermediate edge colors or alpha to the mathematically approved dot envelope.
- [ ] The four-module quiet zone, fixed-canvas background-only padding, exact preview size, deterministic downloads, and payload privacy remain unchanged.
- [ ] Structural, pixel, native/WASM determinism, browser workflow, and pinned independent-decoder tests cover the branded dot output.

