# 08 — Build classified QR function matrices

**What to build:** Construct a checked QR matrix whose function patterns and reserved regions are correctly placed and classified before data placement begins.

**Blocked by:** 04 — Establish and validate QR tables.

**Status:** resolved

- [x] Finder, separator, timing, alignment, format, version, and fixed-dark regions are placed at the required coordinates for every version.
- [x] Versions below 7 omit version information and alignment patterns avoid finder conflicts.
- [x] Every written cell records both its light/dark value and its specific module kind.
- [x] The mutable builder rejects double writes, out-of-bounds coordinates, invalid reservations, and incomplete finalization.
- [x] Human-reviewable fixtures cover Versions 1, 2, 7, and 40, while generated invariants cover all versions.
- [x] Function-coordinate fixtures agree with both pinned public encoders where exposed, cite their exact tagged files/symbols, and retain the `public-corroborated, non-normative` label.
- [x] Matrix construction uses checked, bounds-safe operations and cannot panic on user-controlled input.

## Answer

Added a checked `qr-core::matrix` boundary with immutable classified modules,
coordinate-safe lookup, and a mutable builder that rejects out-of-bounds or
duplicate writes, semantically invalid format/version reservations, and
incomplete finalization. Function construction covers all Model 2 versions,
gives alignment patterns precedence where they cross timing lines, reserves
format/version regions, and classifies the fixed dark module separately.

Committed readable classified maps for Versions 1, 2, 7, and 40. Their
coordinates and stable values are derived independently from intercepted calls
to pinned Nayuki 1.8.0 and python-qrcode 8.2 APIs and are recorded as
`public-corroborated, non-normative`. Generated structural tests check exact
pattern coordinates, values, ownership, and raw data-region accounting for all
40 versions.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, full
Clippy with warnings denied, strict fixture-manifest verification, all Python
support tests, and the pinned function-matrix oracle `--check`. `trunk build
--release` was not applicable because no web, HTML, CSS, WASM boundary, build
configuration, or dependency files changed.
