# 19 — Integrate the bundled magenta ONE lettermark safely

**What to build:** Replace black QR output with a single magenta brand treatment, add the bundled `assets/RGB-one-lettermark-magenta.svg`, and use version-aware safe-area geometry that remains independently decodable at every supported output size.

**Blocked by:** 17 — Add approved color, contrast, and transparency behavior.

**Status:** resolved

- [x] `assets/RGB-one-lettermark-magenta.svg` replaces the gray placeholder as the only logo exposed by the product. It is sanitized and validated at build/test time, embedded without runtime requests, and exposed by no upload or arbitrary-SVG path; the white variant is not used by this task.
- [x] `#BD0F72` is the only QR foreground throughout the product, with or without the logo. Remove black from defaults, selectable presets, configuration, diagnostics, generated combinations, and release evidence; do not retain a hidden black-output path for release 1.
- [x] Opaque white remains the default QR background and required logo knockout so the symbol keeps normal dark-on-light polarity. Existing no-logo transparency may remain available as a caution, but logo mode always uses opaque white and never transparency.
- [x] Enabling the logo switches to ECC H before version fitting and recalculates capacity and diagnostics.
- [x] Logo geometry is calculated after H-level version fitting in QR-module coordinates. It is never sized from the profile canvas alone, because each profile can select multiple matrix sizes.
- [x] Treat the SVG's complete declared `1000×602` view box as the source logo box so its supplied clear space is preserved. Scale it uniformly without clipping, stretching, or independently repositioning its paths, and place an opaque-white knockout behind the complete box.
- [x] The knockout provides at least one full QR module of clear space on every side of the source logo box. Its edges snap outward to module boundaries; rounding may enlarge the knockout but must never reduce the clear space.
- [x] Keep the logo box plus knockout at or below 20% of the selected matrix width. Do not derive an occlusion allowance from ECC H's nominal recovery percentage.
- [x] Prefer exact visual centering. If the centered candidate intersects a protected function module, use a documented deterministic nearest-safe module-grid placement (stable tie-break order) or reject logo mode; never erase a central alignment pattern merely to keep the artwork centered.
- [x] Any intersection with a finder, separator, timing, alignment, format, version, or fixed-dark module is invalid and disables logo export.
- [x] Overlapped data and remainder modules are counted and reported, and valid logo mode remains classified as a caution.
- [x] Commit one generated sizing/placement table covering every selected H-level version available to each output profile: Inline (90 px SVG / 270 px PNG, Versions 1–5), Content (120/360, Versions 1–8), Landing (150/450, Versions 1–12), and Print (160/480, Versions 1–13). Each row records matrix width, module scale, source-logo bounds, knockout bounds, placement offset, protected-module clearance, obscured data/remainder counts, and valid/rejected outcome.
- [x] For each valid row, use the largest candidate that satisfies the 20% cap, one-module clear space, protected-module exclusion, artifact bounds, and independent-decode gate. A smaller tested candidate is required when the largest geometric candidate does not decode reliably; the selected dimensions become compiled, deterministic policy rather than a UI-adjustable logo-size control.
- [x] Unsafe geometry never forces a larger QR version solely to create logo space. The UI reports why logo mode is unavailable for the already-selected version/profile instead of silently changing the payload encoding again.
- [x] Every H-level version allowed by each profile either independently decodes with the logo or produces an intentional, tested geometry rejection.
- [x] SVG and PNG structural tests prove the asset is embedded, its aspect ratio and clear space are preserved, its knockout is opaque white, no black QR modules are emitted in any approved output, and the four-module outer quiet zone contains no logo artwork.
- [x] Documentation records the magenta asset's license/provenance and states that replacing or editing it requires sanitization plus the complete structural, deterministic, geometry, and independent-decode logo suite before release.

## Answer

Resolved on 2026-08-07. Release 1 now emits only `#BD0F72` QR modules. Logo mode refits at ECC H, locks the background/knockout to opaque white, and uses a compiled version-aware policy that preserves the complete ONE source box and avoids every protected function module with deterministic nearest-safe placement.

The generated [placement policy](../../../docs/generated/logo-placement-policy.md) records all 38 profile/version rows. Every row is valid and passed the manifest-pinned ZXing-C++ gate for both production PNG and independently rasterized production SVG. Structural tests cover sanitization, embedding, aspect ratio, knockout/quiet-zone geometry, deterministic artifacts, and absence of black output.
