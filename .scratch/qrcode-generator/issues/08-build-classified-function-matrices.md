# 08 — Build classified QR function matrices

**What to build:** Construct a checked QR matrix whose function patterns and reserved regions are correctly placed and classified before data placement begins.

**Blocked by:** 04 — Establish and validate QR tables.

**Status:** claimed

- [ ] Finder, separator, timing, alignment, format, version, and fixed-dark regions are placed at the required coordinates for every version.
- [ ] Versions below 7 omit version information and alignment patterns avoid finder conflicts.
- [ ] Every written cell records both its light/dark value and its specific module kind.
- [ ] The mutable builder rejects double writes, out-of-bounds coordinates, invalid reservations, and incomplete finalization.
- [ ] Human-reviewable fixtures cover Versions 1, 2, 7, and 40, while generated invariants cover all versions.
- [ ] Function-coordinate fixtures agree with both pinned public encoders where exposed, cite their exact tagged files/symbols, and retain the `public-corroborated, non-normative` label.
- [ ] Matrix construction uses checked, bounds-safe operations and cannot panic on user-controlled input.
