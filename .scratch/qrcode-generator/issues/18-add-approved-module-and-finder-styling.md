# 18 — Add approved module and finder styling

**What to build:** Add only the launch-approved data-module and finder treatments while retaining conservative function geometry and deterministic artifact output.

**Blocked by:** 17 — Add approved color, contrast, and transparency behavior.

**Status:** ready-for-agent

- [ ] Release 1 offers square and rounded data modules; rounded modules never exceed one quarter of a module cell and use deterministic final-pixel coverage without resizing the completed image.
- [ ] Function modules and finders remain square in release 1.
- [ ] Styling changes only rendering geometry and never encoded values, ECC, version, mask, or module classification.
- [ ] SVG paths and PNG coverage remain inside their assigned cells and deterministic across repeated output.
- [ ] Unapproved dot, border, frame, label, stroke, and arbitrary style options are absent from production configuration and UI.
- [ ] Generated tests fail when a new selectable style is not represented in structural, geometry, and independent-decode coverage.
