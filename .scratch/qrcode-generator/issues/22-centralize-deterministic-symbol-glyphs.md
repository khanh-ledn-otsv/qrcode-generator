# 22 — Centralize deterministic symbol glyphs

**What to build:** Preserve the current square QR output while giving SVG and PNG one shared, validated interpretation of visible module glyphs and logo knockout exclusions, so later branded geometry cannot diverge between artifact formats.

**Blocked by:** None — can start immediately.

**Status:** resolved

- [x] One render-model interface classifies visible dark cells in stable row-major order for both SVG and PNG without changing encoded values, module ownership, ECC, version, or mask.
- [x] Finder, separator, other function, data, and remainder ownership remains available to the shared classification without exposing browser APIs or renderer-specific coordinates.
- [x] Logo knockout exclusion is decided once before either artifact adapter paints the surviving glyphs.
- [x] Existing square SVG and PNG artifacts remain byte-for-byte deterministic across repeated native runs and retain every committed structural invariant.
- [x] Native, WASM, and independent-decoder checks prove the prefactor introduces no observable output regression.

## Answer

`RenderModel` now owns a deterministic row-major classification of every module
with finder, separator, other-function, data, and remainder ownership. The same
classification decides the visible `SymbolGlyph` projection and removes
logo-knockout modules before either SVG or PNG paints the symbol. Both adapters
consume that projection, and all existing artifact hashes, structural tests,
WASM parity, and pinned ZXing decode matrices remain unchanged.
