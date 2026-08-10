# 30 — Replace Adaptive Branded with fully adaptive output

**What to build:** Replace the current fixed-size, Version 10-capped **Adaptive
Branded** variant with a selectable **Adaptive** output variant that derives its
QR version, export dimensions, and optional logo placement from the exact
payload. Keep Inline, Content, Landing, and Print available for consumers that
require fixed artifact dimensions.

**Blocked by:** none.

**Type:** task

**Status:** resolved

- [x] `Adaptive Branded` is replaced and presented as `Adaptive`; migrations,
  labels, diagnostics, tests, and documentation consistently use the new
  meaning without silently changing any selected fixed profile.
- [x] With the logo disabled, Adaptive preserves the payload byte-for-byte,
  keeps ECC M, and selects the smallest fitting version through Version 40.
  Payloads that exceed Version 40 capacity receive an exact typed capacity
  error; the existing defensive input ceiling remains independent of QR
  capacity.
- [x] Adaptive derives deterministic SVG and PNG dimensions from the selected
  version and its fixed four-module quiet zone. The reviewed sizing policy
  retains an integer final-pixel module scale of at least six, uses only
  background for surplus canvas pixels, and stays within allocation limits.
- [x] Enabling the bundled logo keeps ECC H and the reviewed branded minimum
  before first-fit version selection. It never lowers ECC, rewrites or shortens
  the URL, or selects a larger version solely to make logo placement easier.
- [x] Logo placement is version-aware: it checks exact center first, then uses
  the deterministic function-safe search policy and reviewed logo legibility
  bounds. Artwork and knockout cells never intersect finder, separator,
  timing, alignment, format, version, or fixed-dark modules.
- [x] Existing decode approval for branded Versions 6–10 is preserved. Every
  higher selected version is enabled for branding only after its production
  placement and final dimensions pass the repository's complete independent
  decode and adverse-transform evidence gate; otherwise logo mode returns a
  typed, user-visible rejection while unbranded Adaptive output remains
  available.
- [x] At least one synthetic ASCII URL that requires a version above 10 at ECC
  H generates a valid branded Adaptive preview and deterministic SVG and PNG
  exports. Boundary fixtures cover the exact fit and one-byte-over transition
  used by that case.
- [x] Adaptive supports the longest payload fitting Version 40 at ECC M without
  a logo. Branded Version 40 is claimed only if its separately reviewed logo
  placement passes all safety and decode gates.
- [x] Diagnostics expose selected version, ECC, derived dimensions, PNG module
  scale, logo bounds/offset when present, obscured data/remainder counts, and a
  clear explanation when branding is unavailable for the selected version.
- [x] Native and WASM output remain byte-identical and deterministic. Approved
  output matrices, resource/allocation baselines, fuzz coverage, browser
  workflows, downloads, accessibility, privacy interception, and pinned
  independent PNG and rasterized-SVG decoding cover the new adaptive size and
  version boundaries.
- [x] `docs/DEVELOPMENT_PLAN.md`, `docs/TESTING_STRATEGY.md`, generated policy
  artifacts, release-readiness guidance, and this implementation map are
  revised consistently before resolution.

## Product intent

Adaptive is the automatic choice for users who care more about reliably
encoding the exact payload than receiving a predetermined pixel size. The four
fixed profiles remain explicit choices for layout contracts and should not be
removed or silently redirected by this ticket.

"Supports long URLs" means that the variant grows through the QR capacity and
dimension ranges instead of stopping at Version 10. It does not promise that an
arbitrary URL fits QR Model 2, nor that every high version is safe for branding
before decode-backed logo evidence exists.

## Implementation constraints

Choose the QR version before calculating render dimensions. Keep encoding in
`qr-core`, adaptive canvas policy and function-safe logo geometry in
`qr-render`, and selection/presentation state in `qr-web`, preserving the
existing dependency direction. Rendering must not alter the selected ECC,
version, mask, or encoded modules.

Do not add URL shortening, payload network requests, analytics, external
assets, or a production QR/image dependency. Do not use ECC percentages as an
occlusion budget. A higher-version logo candidate is a development experiment
until deterministic native/SVG/WASM artifacts independently decode under the
committed evidence policy.

## Comments

## Answer

Replaced the old fixed 180/540, Version 10-capped profile with Adaptive.
Unbranded output now first-fits at ECC M through Version 40 and derives tight
SVG dimensions at two pixels per logical module plus quiet zone and PNG
dimensions at six pixels per logical module plus quiet zone. The exact
Version 40 byte boundary is covered at 2,331 ASCII bytes, with 2,332 bytes
returning the typed capacity failure.

Logo mode still starts at Version 6 and ECC H. The existing placement policy is
decode-approved through Version 11, including a synthetic URL that crosses the
119/120-byte Version 10-to-11 boundary. Versions 12–40 return a typed,
user-visible logo rejection that instructs the user to disable the logo while
preserving the exact payload. Fixed profiles remain unchanged and selectable.

The generated output matrix now contains 436 scenarios: 254 decoded and 182
typed expected rejections. Pinned ZXing independently decoded every enabled
native PNG and selected-version rasterized SVG row, including all Adaptive
versions through Version 40 and branded Versions 6–11. The adverse manifest
records 39 outcomes across five envelopes; the compact Adaptive Version 10 and
11 artifacts each pass their documented five-transform caution envelope.
