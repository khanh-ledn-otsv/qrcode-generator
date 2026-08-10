# QR Code Generator — Implementation map

## Decisions-so-far

- [Ticket 02](issues/02-define-output-profiles-and-geometry.md): Output profiles
  are compiled constants; QR versions belong to `qr-core`; and `qr-render`
  calculates checked, centered, background-only fixed-canvas geometry.
- [Ticket 03](issues/03-establish-fixtures-and-independent-oracles.md): Strict
  fixture provenance and hashes are checked by a development-only Rust tool;
  pinned Nayuki and `python-qrcode` generators must agree on explicit matrices;
  and pinned ZXing-C++ independently checks raster payload bytes and metadata.
- [Ticket 04](issues/04-transcribe-and-validate-qr-tables.md): Complete QR Model
  2 capacity, block, remainder, alignment, and character-width data is exposed
  through typed checked lookups and exhaustively validated against a committed
  160-row fixture from pinned independent development oracles.
- [Ticket 05](issues/05-encode-payloads-into-data-codewords.md): Exact input is
  encoded as one deterministic Numeric, Alphanumeric, or Byte segment with
  UTF-8 ECI 26 when required, first-fit version selection, typed failures, and
  fully padded data codewords.
- [Ticket 08](issues/08-build-classified-function-matrices.md): Checked matrix
  construction classifies every function region for all 40 versions, validates
  reservations and finalization, and is covered by exact all-version invariants
  plus dual-oracle readable fixtures for Versions 1, 2, 7, and 40.
- [Ticket 09](issues/09-place-data-remainder-and-explicit-masks.md): Checked
  zig-zag placement preserves data/remainder ownership, protects every function
  module, supports all eight explicit masks through a typed core API, and
  matches dual-oracle fixtures at Versions 1, 2, 7, and 40.
- [Ticket 12](issues/12-create-safe-render-model.md): A borrowed immutable
  render model preserves encoded ownership and diagnostics, centralizes the
  exact quiet-zone extent, separates tight SVG from fixed-canvas PNG placement,
  checks allocation bounds, and exposes only typed data/remainder branding
  targets.
- [Ticket 13](issues/13-export-safe-svg-artifacts.md): A dependency-free
  production exporter emits compact deterministic safe SVG with exact logical
  placement; parsed all-profile/version structure, independent rasterization,
  and pinned ZXing decode prove the artifact boundary.
- [Ticket 14](issues/14-export-safe-png-artifacts.md): A pinned direct PNG
  exporter paints checked RGBA rectangles, emits deterministic metadata-free
  artifacts, matches a native/WASM hash contract, and passes parsed-pixel plus
  pinned ZXing decode gates across every supported profile and version.
- [Ticket 15](issues/15-deliver-safe-interactive-workflow.md): A plain-Rust,
  revisioned workflow state machine owns exact payload/profile fitting and
  validation, while the Leptos view schedules cancellable preview
  work and exposes safe fixed-ECC diagnostics without allowing stale results.
- [Ticket 16](issues/16-complete-downloads-accessibility-and-privacy.md): Ready
  previews own deterministic SVG/PNG downloads with bounded browser resources;
  complete payload-free diagnostics and accessible states are backed by native,
  WASM, privacy-interception, hash, and pinned-decode tests; browser release
  coverage is now desktop Chromium only.
- [Ticket 17](issues/17-add-approved-color-and-transparency.md): The contrast and
  transparency infrastructure validates compiled appearance choices, placement
  cautions, and generated structural/deterministic/pinned-decode coverage.
- [Ticket 18](issues/18-add-approved-module-and-finder-styling.md): The original
  rounded-module option was subsequently removed; release output now uses only
  square data/function modules and standard square finders.
- [Ticket 19](issues/19-integrate-bundled-logo-safely.md): Magenta is the sole QR
  foreground; the sanitized ONE lettermark is compile-time embedded behind an
  opaque-white, function-safe knockout selected by deterministic H-level module
  geometry, with all profile/version rows passing pinned SVG and PNG decoding.
- [Ticket 20](issues/20-harden-approved-output-combinations.md): A generated
  96-row matrix records all approved output/payload combinations and typed
  rejections, while deterministic adverse decoding, quantitative coverage and
  mutation gates, fuzz/Miri/dependency commands, and reproducible allocation
  and artifact baselines form the release evidence; performance benchmarks were
  subsequently removed.
- [Ticket 21](issues/21-validate-release-readiness.md): The automated release
  gate proves reproducible build hashes, zero-retry browser workflows, privacy,
  downloads, approved artifacts, and guidance; manual product checks are kept
  outside the repository evidence system.
- [Ticket 22](issues/22-centralize-deterministic-symbol-glyphs.md): SVG and PNG
  now consume one row-major visible-glyph classification with shared ownership
  and logo-knockout exclusion while retaining byte-identical square artifacts.
- [Ticket 23](issues/23-approve-decode-backed-branded-geometry.md): Pinned
  independent decoding approved 0.45-module centered non-finder dots, full-cell
  square finders, a Version 6 branded minimum, and a 13×4.875-module exactly
  centered ONE source with a function-safe 15×7 white knockout; Versions 7–13
  intentionally reject centered-logo geometry.
- [Ticket 24](issues/24-render-compact-dots-with-square-finders.md): Production
  SVG and deterministic direct-RGBA PNG now render every visible non-finder
  module as an exact centered 0.45-module dot while retaining full-cell square
  finders, blank separators, unchanged placement/privacy, and a single compiled
  appearance.
- [Ticket 25](issues/25-select-branded-minimum-version-in-logo-mode.md): Checked
  encoder ranges preserve ordinary first fitting while logo mode requests ECC H
  and at least Version 6; diagnostics identify branding-enlarged symbols and
  profiles below the branded minimum receive a specific validation failure.
- [Ticket 26](issues/26-enlarge-exactly-centered-one-logo.md): Production SVG,
  PNG, previews, diagnostics, and downloads now share the single decode-backed
  Version 6 centered 13×4.875-module ONE placement and 15×7 knockout; every
  other version receives a typed, user-visible geometry rejection.
- [Ticket 27](issues/27-harden-complete-branded-output-matrix.md): The generated
  matrix pairs native PNG and independently rasterized SVG hashes,
  pinned-decoder outcomes, typed rejections, and exact logo geometry across all
  selectable payload/version paths; 29 manifest-owned adverse outcomes and
  zero-retry Chromium workflows complete the automated release evidence.
- [Ticket 28](issues/28-increase-inline-size-for-version-six-logo.md): Inline is
  now 100 px SVG / 300 px PNG with a Version 6 ceiling and six-pixel PNG module
  scale, so fitting logo-mode payloads use the approved centered ONE treatment
  instead of ending in a profile/version compatibility failure.
- [Ticket 29](issues/29-support-adaptive-large-branded-qr.md): Adaptive Branded
  is a separate non-default 180 px SVG / 540 px PNG profile through Version 10;
  its deterministic function-safe placement keeps the largest reviewed ONE
  treatment, lets the exact long URL export at ECC H / Version 10, and is backed
  by 316-row dual-format decoder evidence plus a 35-outcome adverse manifest.
- [Ticket 30](issues/30-replace-adaptive-branded-with-fully-adaptive-output.md):
  Adaptive now derives selected-version SVG/PNG dimensions and supports exact
  unbranded ECC-M payloads through Version 40; logo mode remains ECC H and is
  independently decode- and adverse-approved through Version 11, with typed
  rejection above that boundary and all four fixed profiles preserved.
- [Ticket 31](issues/31-increase-dot-contrast-and-document-link-capacity.md):
  Compact dots now use a decode-backed 0.75-module circle with a solid
  ONE-magenta core plus an antialiased contour in SVG and PNG; the practical guide publishes workflow-owned,
  boundary-tested ASCII Byte-mode limits for every profile with and without the
  logo plus variant-selection and Adaptive sizing/logo-placement tradeoffs,
  backed by refreshed deterministic and independent-decode evidence.

## Notes

- Implementation tickets are tracked under `issues/`.

## Fog

- [Ticket 32](issues/32-simplify-output-and-automate-correctness.md): Fix the CI
  caches and pinned tools, add routine and extended hosted correctness gates,
  add the documented quirc decoder, remove transparent output, and make the
  ordinary opaque output path use unconditional Rounded ONE modules with the
  bundled logo enabled by default and removable. The licensed ISO audit and
  physical-device validation remain owner-dependent work outside the ticket.
- [Ticket 10 penalty-oracle disagreement](penalty-oracle-disagreement.md):
  Nayuki 1.8.0 and python-qrcode 8.2 agree on completed matrices but disagree
  on exposed Rule 3 penalty totals, blocking fixture acceptance under the
  public-source provenance policy.
