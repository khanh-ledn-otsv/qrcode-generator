# 31 — Increase compact-dot contrast and document link capacity

**What to build:** Render the compact QR dots with the same ONE magenta used by
the bundled logo and prominent square finder patterns, instead of a lighter
dot treatment, and add an in-product guide showing the maximum typical link
length for every output variant with and without the logo.

**Blocked by:** none.

**Type:** task

**Status:** resolved

- [x] SVG, PNG, the on-page preview, and downloaded artifacts use the exact
  approved brand foreground `#BD0F72` for compact dots, the bundled logo, and
  the three prominent square finder patterns. Compact dots have no separate
  lighter color token, reduced opacity, filter, or compositing treatment.
- [x] The approved centered 0.45-module dot geometry, square finder geometry,
  module ownership, quiet zone, logo knockout, payload encoding, selected QR
  version, mask, and ECC policy remain unchanged unless new decode evidence
  explicitly revises the corresponding policy.
- [x] Deterministic PNG coverage keeps fully covered dot pixels at the exact
  brand RGB. Intermediate RGB or alpha is permitted only at the mathematical
  anti-aliased contour of a dot; it must not lighten the dot interior or act as
  a second foreground color. Transparent output retains brand RGB beneath
  contour alpha.
- [x] The practical guide contains a clear, accessible table for all five
  variants. For a typical ASCII URL that selects QR Byte mode, it shows these
  maximum total payload lengths, including the scheme, host, path, query, and
  fragment:

  | Output variant | Without logo | With logo |
  | --- | ---: | ---: |
  | Inline | 106 characters / bytes | 58 characters / bytes |
  | Content | 152 characters / bytes | 58 characters / bytes |
  | Landing | 287 characters / bytes | 58 characters / bytes |
  | Print | 331 characters / bytes | 58 characters / bytes |
  | Adaptive | 2,331 characters / bytes | 137 characters / bytes |

- [x] The guide explains why the columns differ: no-logo output uses ECC M;
  logo output uses ECC H; the fixed variants approve branded geometry only at
  Version 6; and Adaptive currently approves branded placement through Version
  11. It must not imply that the logo merely subtracts a fixed number of
  characters or that ECC H's nominal percentage is an occlusion budget.
- [x] The guide labels these as Byte-mode ASCII link limits, not universal
  Unicode character limits. ASCII characters are one UTF-8 byte each;
  non-ASCII characters can consume multiple bytes plus ECI overhead, while a
  payload containing only the QR alphanumeric set can sometimes fit more. The
  current preview/validation result remains authoritative for the exact text
  entered.
- [x] Capacity values are derived from, generated from, or exhaustively checked
  against the same compiled profile ceilings, ECC selection, logo-geometry
  approval, and `qr-core` fit behavior used by production. A profile or logo
  policy change must fail a test until the guide is updated; do not maintain an
  unverified second capacity table.
- [x] Exact-fit and one-byte-over tests use synthetic ASCII URLs at every table
  boundary. They prove that the stated maximum succeeds and the next byte
  produces the appropriate capacity or logo-policy rejection without trimming,
  shortening, normalizing, logging, or transmitting the payload.
- [x] Parsed SVG and PNG pixel tests prove common brand color usage and confine
  contour blending to the approved dot envelope. Deterministic artifact hashes,
  native/WASM equality, the approved output matrix, adverse-transform evidence,
  and pinned independent SVG/PNG decoding are refreshed where the raster change
  affects them.
- [x] Desktop Chromium verifies the visible capacity guide, semantic table
  structure, keyboard/screen-reader accessibility, higher-contrast preview,
  deterministic downloads, and zero payload or logo network requests.
- [x] `docs/DEVELOPMENT_PLAN.md`, `docs/TESTING_STRATEGY.md`, generated
  appearance/evidence artifacts, release guidance, and the implementation map
  are updated consistently before resolution.

## Product intent

The compact dots should read as the same strong ONE magenta as the logo and
finder patterns. This is a foreground-contrast change, not a request to enlarge
the dots, weaken the quiet zone, alter protected function modules, or change QR
encoding.

The capacity guide should answer a planning question before users paste a link,
while remaining honest about QR modes and UTF-8. “Maximum link length” means the
total encoded payload length under the stated Byte-mode ASCII assumption; the
application still preserves and evaluates the exact input.

## Implementation constraints

Keep encoding and capacity decisions in `qr-core`, rendering and deterministic
pixel coverage in `qr-render`, and presentation/accessibility in `qr-web`.
Keep all payload processing in the browser. Do not add URL parsing that rewrites
input, URL shortening, production network requests, analytics, external fonts,
or a second QR implementation in production.

Changing raster coverage changes approved artifacts and therefore requires the
same structural, deterministic, independent-decode, and adverse-transform
review used for the current compact-dot appearance. Do not weaken or silently
regenerate golden evidence to accept the new output.

## Comments

- Follow-up: expanded the practical guide so users can compare the intended
  placement, fixed dimensions, and ceiling of every variant. Adaptive now
  explains first-fit sizing, its variable artifact dimensions, its centered
  Version 6 logo, the upward-shifted Versions 7–11 placement, and the typed
  logo rejection at Version 12 or higher.
- Regression fix: removed forced crisp-edge SVG rendering and restored
  deterministic PNG contour coverage after those treatments made the compact
  circles display as diamonds/squares. Both formats retain an exact solid brand
  core with antialiasing confined to the mathematical circular contour.

## Answer

Compact non-finder modules now use the exact ONE-magenta fill in both SVG and
PNG while retaining their round appearance. SVG uses its true circular arcs
with normal antialiasing; PNG keeps an exact solid brand core and deterministic
8-by-8 contour coverage. Finder geometry, quiet zones, knockout, encoding,
version selection, masks, and ECC policy are unchanged.

The practical guide now contains a semantic five-row table for Inline,
Content, Landing, Print, and Adaptive, with separate no-logo and logo limits.
The values are exposed by the workflow layer and exhaustively checked at exact
fit and one byte over against production encoding and logo-geometry policy.
The guide explains ECC and reviewed-version differences, ASCII Byte-mode and
UTF-8/ECI qualifications, and makes the live preview authoritative.
It also recommends when to choose each fixed variant versus Adaptive and makes
Adaptive's payload-derived dimensions and non-centered higher-version logo
placement explicit.

Deterministic fixture hashes and resource baselines were refreshed explicitly.
Pinned independent PNG and rasterized-SVG decoding passed the complete output
matrix, the 39-outcome adverse envelope passed, and `pnpm run verify` passed,
including native/WASM Rust tests, linting, release build, Python checks, and
20 desktop Chromium tests with privacy interception.
