# 18 — Add approved module and finder styling

**What to build:** Add only the launch-approved data-module and finder treatments while retaining conservative function geometry and deterministic artifact output.

**Blocked by:** 17 — Add approved color, contrast, and transparency behavior.

**Prerequisite:** The owner has confirmed whether rounded data modules or a named finder preset ships; unapproved dot modules remain unavailable.

**Status:** ready-for-agent

- [ ] Approved rounded data modules never exceed one quarter of a module cell and use deterministic final-pixel coverage without resizing the completed image.
- [ ] Function modules remain square unless a separately approved and named finder preset explicitly changes finder geometry.
- [ ] Styling changes only rendering geometry and never encoded values, ECC, version, mask, or module classification.
- [ ] SVG paths and PNG coverage remain inside their assigned cells and deterministic across repeated output.
- [ ] Unapproved dot, border, frame, label, stroke, and arbitrary style options are absent from production configuration and UI.
- [ ] Generated tests fail when a new selectable style is not represented in structural, geometry, and independent-decode coverage.
