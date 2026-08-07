# 22 — Centralize deterministic symbol glyphs

**What to build:** Preserve the current square QR output while giving SVG and PNG one shared, validated interpretation of visible module glyphs and logo knockout exclusions, so later branded geometry cannot diverge between artifact formats.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] One render-model interface classifies visible dark cells in stable row-major order for both SVG and PNG without changing encoded values, module ownership, ECC, version, or mask.
- [ ] Finder, separator, other function, data, and remainder ownership remains available to the shared classification without exposing browser APIs or renderer-specific coordinates.
- [ ] Logo knockout exclusion is decided once before either artifact adapter paints the surviving glyphs.
- [ ] Existing square SVG and PNG artifacts remain byte-for-byte deterministic across repeated native runs and retain every committed structural invariant.
- [ ] Native, WASM, and independent-decoder checks prove the prefactor introduces no observable output regression.

