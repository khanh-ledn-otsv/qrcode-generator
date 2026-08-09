# 23 — Approve decode-backed branded geometry

**What to build:** Turn the reference appearance into a measured production policy by selecting the smallest reliable centered dots, the safe non-dot function patterns, the minimum branded logo version, and the largest reliable exactly centered ONE logo.

**Blocked by:** 22 — Centralize deterministic symbol glyphs.

**Status:** resolved

- [x] A deterministic development experiment compares dot diameters from 0.45 through 0.60 module without changing encoded module values or positions.
- [x] The experiment compares conservative square function patterns with the closer-reference non-finder dot treatment and admits only geometry that passes the complete independent decode sample.
- [x] Logo candidates use exact matrix centering, opaque-white knockout geometry, ECC H, checked protected-module clearance, and recorded obscured data/remainder counts.
- [x] The candidate set evaluates a branded minimum version large enough to produce the requested visual hierarchy without rewriting, padding, normalizing, or transmitting the payload.
- [x] SVG-rasterized and native PNG candidates cover every supported profile, required payload class, enabled logo version, and intentional unsafe-geometry rejection.
- [x] The authoritative development and testing policies record the selected dot diameter, function-pattern treatment, minimum logo version, logo-size table, quiet-zone rule, and exclusion of decorative export borders.

## Answer

The manifest-pinned ZXing-C++ 3.0.2 sweep approved exactly centered
0.45-module dots with full-cell square finders and dots for every other visible
module. All 32 diameter/treatment candidates passed their complete 96-artifact
sample, so 0.45 is the smallest tested reliable diameter and the
closer-reference non-finder-dot treatment is selected.

Logo mode selects at least Version 6. Versions 4 and 5 admit only a 10-module
function-safe centered source, while Version 6 is the first to admit the
requested 12-module hierarchy. The selected Version 6 ONE source is 12×4.5
modules; its opaque-white knockout is `(13, 17) 15×7`, clears protected modules
by six modules, obscures 105 data and zero remainder modules, and decoded 36/36
native-PNG and rasterized-SVG cases across Content, Landing, and Print. Inline
intentionally rejects logo mode because its Version 5 ceiling is below the
branded minimum. Larger tested widths violate the checked
40% knockout bound. Versions 7–13 are intentional exact-centering rejections
because of protected central alignment geometry. Four quiet modules remain
mandatory and decorative export borders remain excluded. Full evidence is in
[`docs/generated/branded-geometry-policy.json`](../../../docs/generated/branded-geometry-policy.json).
