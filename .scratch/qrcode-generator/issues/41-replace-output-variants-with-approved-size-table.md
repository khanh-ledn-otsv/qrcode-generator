# 41 — Replace output variants with approved size table

**What to build:** Replace the current selectable variants, including Adaptive,
with the approved fixed-size Digital and Print variants. Treat the researched
size/version table as an input to validate, not as final product truth, and
convert print sizes from millimeters to pixels with a 150 dpi policy.

**Blocked by:** 40

**Type:** task

**Status:** resolved

- [x] Remove Adaptive from the public selectable variant list, migrations,
  labels, diagnostics, tests, documentation, and approved-output matrix.
- [x] Provide exactly these Digital variants and use their researched version
  ranges as starting candidates, then confirm or adjust those ranges with the
  final module pitch, logo geometry, and independent decoder evidence:
  - Small: 100 x 100 px, intended for web footer and secondary CTA usage, QR
    Version >= 5.
  - Standard: 120 x 120 px, intended for general web content, QR Versions 5-8.
  - Primary CTA: 160 x 160 px, intended for download-app and continue-on-mobile
    usage, QR Versions 5-12.
  - Hero / Campaign: 200 x 200 px, intended for landing-page and campaign
    usage, QR Versions 8-12.
- [x] Provide exactly these Print variants, converting millimeters at 150 dpi
  and rounding to the nearest integer pixel:
  - Business card: 25 mm -> 148 x 148 px.
  - Flyer / Brochure: 30 mm -> 177 x 177 px.
  - Poster / Package: 40 mm -> 236 x 236 px.
- [x] Keep export dimensions deterministic and identical between preview, SVG,
  PNG, diagnostics, approved matrices, browser downloads, and documentation.
- [x] Keep physical-size guidance explicit: the pixel conversion is a 150 dpi
  artifact policy, while real print output still requires owner/device testing
  on the final material and surface.
- [x] Document the accepted module-pitch rule used to approve final version
  ranges. For a Version `v` symbol with a four-module quiet zone, the total
  logical width is `4v + 25` modules, so each fixed artifact size must leave a
  scannable per-module pitch after quiet zone and logo constraints.
- [x] Preserve exact payload handling, local-only browser processing, opaque
  white background, and the two approved foreground/logo themes.
- [x] Run the routine covering gate plus affected approved-output, artifact hash,
  and independent decoder checks for all retained variants.

## Product intent

The app should offer concrete, named output choices that match real placement
needs. Users should not have to understand Adaptive sizing to pick an artifact;
they choose the variant that matches their destination. The researched table is
a useful draft, but final support should follow measurable scan reliability and
not merely repeat the screenshot.

## Implementation constraints

Select a QR version before calculating render geometry. Fixed variant dimensions
are hard output contracts; if a payload cannot fit within a variant's allowed
version range and logo policy, return a typed user-visible failure rather than
silently changing the payload or generating a non-contract size.

## Comments

## Answer

Replaced the legacy selectable profiles with the seven approved fixed Digital
and Print variants. The workflow selects within each explicit range before
render geometry is calculated; unsupported logo placement returns a typed
visible failure. Print artifacts use the required 150 dpi conversions (148,
177, and 236 px), and preview, SVG, PNG, diagnostics, downloads, approved
matrix, and generated logo policy share each compiled size. The accepted pitch
rule is $4v + 25$ logical modules, with a centered integer pitch and at least
six PNG pixels per module at each approved maximum version.
