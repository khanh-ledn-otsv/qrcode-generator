# 42 — Allow profile version selection with typed failures

**What to build:** Let each approved variant select the QR version it needs
within an evidence-backed version policy, instead of assuming one fixed version.
Version changes are allowed only to make the exact payload fit safely within the
selected variant.

**Blocked by:** 41

**Type:** task

**Status:** open

- [ ] For each variant, first-fit the exact payload using the variant's allowed
  QR version range, ECC policy, foreground theme, and logo setting. Do not trim,
  normalize, shorten, rewrite, URL-parse, log, or transmit the payload.
- [ ] Use the researched Digital ranges as provisional candidates only: Small
  starts at Version 5, Standard candidates are Versions 5-8, Primary CTA
  candidates are Versions 5-12, and Hero / Campaign candidates are Versions
  8-12. Adjust these ranges if module pitch, logo safety, or decoder evidence
  shows they are too narrow or too ambitious.
- [ ] Define and document the Print variant version policies explicitly before
  implementation, using the same scan-readability and physical-size rationale as
  the Digital variants. Do not infer undocumented print ranges in production,
  and do not copy Digital ranges when the 150 dpi physical size produces a
  materially different module pitch.
- [ ] For every candidate range, calculate the module pitch from final artifact
  width divided by `4v + 25` modules, including the four-module quiet zone. Use
  that pitch, rounded-dot readability, logo obstruction, and decoder evidence to
  decide the maximum version for each fixed size.
- [ ] Preserve the existing ECC transition semantics unless deliberately revised
  in the authoritative product plan: no-logo output uses ordinary ECC capacity;
  logo output uses the reviewed high-ECC branded path.
- [ ] If the exact payload does not fit any allowed version for the chosen
  variant and logo policy, return the existing typed capacity/profile failure
  with selected variant, attempted version range, ECC, and payload capacity
  diagnostics.
- [ ] Ensure version selection remains in `qr-core`/workflow ownership and that
  `qr-render` only consumes an immutable encoded matrix.
- [ ] Cover exact-fit and one-over boundaries for every variant, both approved
  foreground themes, and logo on/off states.

## Product intent

The app can change QR version when needed because version is a capacity and
readability decision, not a visible styling knob. A variant still owns the final
artifact size and acceptable version envelope. Final envelopes should be tighter
than the QR standard's theoretical capacity when the fixed output size, rounded
modules, or logo would make scanning fragile.

## Implementation constraints

Do not select a larger QR version solely to make logo placement easier unless an
accepted product decision explicitly says that variant permits it. Rendering
must not change ECC, version, mask, or encoded modules after the workflow has
selected them.

## Comments
