# 18 — Add approved module and finder styling

**What to build:** Add only the launch-approved data-module and finder treatments while retaining conservative function geometry and deterministic artifact output.

**Blocked by:** 17 — Add approved color, contrast, and transparency behavior.

**Status:** resolved

- [x] Release 1 offers square and rounded data modules; rounded modules never exceed one quarter of a module cell and use deterministic final-pixel coverage without resizing the completed image.
- [x] Function modules and finders remain square in release 1.
- [x] Styling changes only rendering geometry and never encoded values, ECC, version, mask, or module classification.
- [x] SVG paths and PNG coverage remain inside their assigned cells and deterministic across repeated output.
- [x] Unapproved dot, border, frame, label, stroke, and arbitrary style options are absent from production configuration and UI.
- [x] Generated tests fail when a new selectable style is not represented in structural, geometry, and independent-decode coverage.

## Answer

Resolved on 2026-08-07. Release 1 now exposes exactly square and rounded data
modules from one compiled approved-style list. Rounded SVG paths use a fixed
quarter-cell radius, while PNG output computes deterministic 8-by-8 subpixel
coverage directly at the final output resolution. Protected function modules,
including standard finder patterns, retain full square cells in both formats.

The Leptos workflow defaults to square modules and offers keyboard-native
Square and Rounded controls with explicit diagnostics for data, function, and
finder geometry. Styling remains a render-only choice: model and workflow tests
prove that encoded modules, classification, ECC, version, and mask are
unchanged.

Generated structural, geometry, and pinned ZXing-C++ decode suites enumerate
the same approved-style list used by production. The complete Node 24
`pnpm run verify` suite passed, along with explicit ignored-test runs of the
approved SVG and PNG independent-decode matrices.
