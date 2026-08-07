# 18 — Add approved module and finder styling

**What to build:** Add only the launch-approved data-module and finder treatments while retaining conservative function geometry and deterministic artifact output.

**Blocked by:** 17 — Add approved color, contrast, and transparency behavior.

**Status:** resolved

- [x] Release 1 offers square data modules only.
- [x] Function modules and finders remain square in release 1.
- [x] Styling changes only rendering geometry and never encoded values, ECC, version, mask, or module classification.
- [x] SVG paths and PNG coverage remain inside their assigned cells and deterministic across repeated output.
- [x] Unapproved dot, border, frame, label, stroke, and arbitrary style options are absent from production configuration and UI.
- [x] Generated tests fail when a new selectable style is not represented in structural, geometry, and independent-decode coverage.

## Answer

Release 1 now exposes square QR modules and standard square finders only.
Production configuration and the Leptos workflow contain no module-shape
control or alternate rendering branch. Structural and independent-decode suites
cover the remaining deterministic square treatment.

## Comments

- 2026-08-07: The owner removed rounded-module support after the original
  resolution. Production configuration, UI, rendering branches, and rounded
  geometry tests were removed; square modules remain the sole treatment.
