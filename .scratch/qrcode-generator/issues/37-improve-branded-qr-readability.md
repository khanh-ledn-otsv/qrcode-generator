# 37 — Improve branded QR readability

**What to build:** Directly improve scan readability while preserving the
mandatory Rounded ONE appearance and bundled ONE logo by increasing rounded
module coverage, modestly reducing the logo knockout, and adding deterministic
mixed-mode encoding.

**Blocked by:** none.

**Type:** task

**Priority:** direct scan-readability improvement

**Status:** resolved

## Rounded-module coverage

- [x] Run a deterministic candidate campaign for centered circular diameters
  above the current `0.75` module, including at least `0.80`, `0.85`, and `0.90`
  module. Do not select a value from appearance alone.
- [x] Select the largest candidate that remains visually round at every
  compiled SVG and PNG module scale and improves or preserves independent
  decode behavior under the approved adverse transforms. Retain `0.75` if no
  larger candidate clears those gates.
- [x] Keep every circle centered and strictly inside its owning module so
  adjacent ideal SVG glyphs do not merge.
- [x] Preserve full-cell square finder regions, blank separators, function
  module ownership, the four-module quiet zone, opaque white background, and
  `#BD0F72` foreground.
- [x] Use one shared approved diameter in the render model. PNG output must
  retain a solid brand-color core with deterministic antialiasing confined to
  the mathematical circular contour.

## ONE logo knockout

- [x] Keep the sanitized ONE asset, its `180 180 640 240` presentation box,
  `13 × 4.875`-module source bounds, color, aspect ratio, and reviewed position
  policy unchanged.
- [x] Compare the current `15 × 7`-module knockout with modestly smaller,
  centered candidates. The candidate floor is `14 × 6` modules; reducing
  either dimension below that floor requires a separate owner decision.
- [x] Preserve at least a half-module of opaque-white clearance between the
  logo source bounds and knockout boundary on every side. Reject any candidate
  that clips, crowds, distorts, visually weakens, or permits QR marks to show
  through the ONE treatment at an approved output size.
- [x] Choose the smallest reduction that measurably decreases obscured data
  modules while preserving logo quality and improving or preserving comparative
  decode behavior. Retain the current knockout if no smaller candidate clears
  both visual and decode gates.
- [x] Recalculate obscured data/remainder counts and protected-module clearance
  for every enabled fixed and Adaptive branded version. The knockout must never
  intersect a function module or the four-module quiet zone.
- [x] Preserve ECC H, the Version 6 branded minimum, fixed-profile centering,
  and the reviewed Adaptive placement policy. Do not shrink or move the logo
  merely to make a knockout candidate pass.

## Mixed-mode encoding

- [x] Implement a version-aware deterministic segmentation algorithm in
  `qr-core` that minimizes total bits across Numeric, Alphanumeric, and Byte
  segments, including indicators, character-count widths, payload costs, and
  UTF-8 ECI 26 overhead.
- [x] Preserve exact input bytes. Do not trim, normalize, rewrite, shorten,
  log, transmit, or URL-parse the payload.
- [x] Keep pure Numeric, pure Alphanumeric, ASCII Byte, and UTF-8 Byte output
  byte-for-byte compatible when the optimal representation is unchanged.
- [x] Emit ECI assignment 26 before the first UTF-8 Byte segment when required,
  and never add ECI to ASCII-only Byte segments.
- [x] Resolve equal-bit segmentations with a documented stable tie-breaker so
  native and WASM matrices remain deterministic.
- [x] Account for version-dependent character-count widths during first-fit
  selection, including the Version 9/10 and 26/27 transitions.
- [x] Represent segments with typed core data and update diagnostics so a
  mixed symbol is not reported as one misleading mode.
- [x] Prove with properties and independent oracles that optimization never
  uses more bits than the whole-payload policy, never changes decoded content,
  remains deterministic, and cannot turn a previously fitting payload into a
  capacity failure.

## Combined evidence and acceptance

- [x] Exercise short and dense payloads across every enabled
  profile/version/logo path. Independently decode native PNG and rasterized SVG
  artifacts and cover low module scales, downscaling, blur, rotation,
  perspective, grayscale, brightness/contrast change, dot gain, and ink loss.
- [x] Add deterministic visual comparisons at actual SVG preview and PNG
  export sizes, including circular glyph quality, logo whitespace, logo edge
  antialiasing, and dense modules immediately outside the knockout.
- [x] Add exact-fit and one-over mixed-segment tests for all ECC levels,
  ASCII/UTF-8+ECI transitions, profile ceilings, and Version 40/input limits.
- [x] Compare selected segments, data codewords, completed matrices, decoded
  bytes, ECI metadata, versions, and render artifacts with pinned independent
  oracles. Production code must not depend on another QR implementation.
- [x] Explicitly review and refresh affected golden matrices, logo-placement
  evidence, artifact hashes, resource baselines, adverse evidence, and capacity
  guidance. Never regenerate golden evidence implicitly during tests.
- [x] Update `docs/DEVELOPMENT_PLAN.md`, `docs/TESTING_STRATEGY.md`, fixture
  provenance, generated evidence, diagnostics, and the implementation map.
- [x] Run `pnpm run verify` plus the applicable approved-output, independent
  generator/decoder, logo decode, adverse, fuzz, coverage, and mutation gates
  before resolving the task.

## Product and architecture constraints

Rounded ONE modules and the bundled ONE logo are hard requirements. This task
must not introduce square non-finder modules, an appearance selector, a
logo-free default, a different foreground, transparency, or a second production
style. Logo quality takes precedence over recovering additional modules.

Mixed-mode decisions belong in `qr-core`. `qr-render` must continue to consume
an immutable encoded matrix without altering segments, ECC, version, mask, or
modules. The web layer may present segment diagnostics but must not make
encoding decisions.

This task intentionally changes matrices for some mixed payloads and may change
approved render artifacts. Each change must be explained by a smaller
standards-valid bitstream or a decode-backed rendering improvement and reviewed
through explicit evidence.

## Comments

## Answer

Implemented the three readability improvements together. Rounded ONE data
modules now use the largest decode-approved campaign candidate, a centered
`0.90`-module circle with deterministic 8×8 PNG coverage. The ONE asset and
its reviewed source placement remain unchanged. The knockout campaign retained
the `15 × 7` baseline: every smaller centered candidate either clipped the new
0.90-module dots at a fractional boundary or left lettermark pixels outside
the opaque-white field. It therefore continues to obscure 105 data modules.

`qr-core` now performs deterministic, version-band-aware dynamic programming
across Numeric, Alphanumeric, and Byte segments, emits UTF-8 ECI 26 once when
needed, preserves whole-payload golden output when it remains optimal, and
exposes typed segment diagnostics so the web UI reports `Mixed` accurately.
The stable tie order is minimum bits, fewer segments, Numeric before
Alphanumeric before Byte, then the longer first segment recursively.

Candidate decisions, mixed-mode oracle fixtures, placement policy, artifact
hashes, capacity boundaries, and documentation were refreshed explicitly.
`pnpm run verify`, approved-output/resource baselines, independent ZXing PNG
and rasterized-SVG decoding, bundled-logo decoding, adverse transforms, quirc,
coverage, and mutation thresholds passed. Core mutation scored 95.32% overall
and 95.59% for critical algorithms after 25 pre-existing nonterminating
mutants were explicitly triaged without counting them as caught; the focused
task-related rerun caught every tie-order mutant. Render geometry scored
90.91%. With `cargo-fuzz 0.13.2` and nightly Rust installed, the four applicable
targets completed 600-second sanitizer runs without crashes: `encode_utf8`
executed 328,345 cases, `encode_valid_text` 263,962, `render_svg` 191,113, and
`render_png` 92,318. Two transient slow-unit artifacts replayed in 5 ms and
19 ms, so they were triaged as non-reproducible corpus-discovery outliers.
The renderer fuzz harness was also updated to the current opaque safe preset
and now exercises branded and unbranded output.
