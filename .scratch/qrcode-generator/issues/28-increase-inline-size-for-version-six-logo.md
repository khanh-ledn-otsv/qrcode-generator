# 28 — Increase Inline size for the Version 6 logo

**What to build:** Increase the Inline output profile to a 100 px SVG and
300 px PNG so it can admit Version 6 and retain the approved bundled ONE logo
without reducing the six-pixel PNG module scale.

**Blocked by:** none.

**Type:** task

**Status:** resolved

- [x] Inline uses a 100×100 px SVG canvas, a 300×300 px PNG canvas, and a
  Version 6 ceiling everywhere the profile is presented or validated.
- [x] Inline Version 6 retains an integer scale of at least six PNG pixels per
  module, four quiet-zone modules per side, and symmetric background-only
  surplus padding.
- [x] Logo mode on Inline continues to use ECC H, the Version 6 branded
  minimum, the existing exactly centered `13 × 4.875`-module ONE artwork, and
  its approved 15×7 opaque-white knockout; no smaller Inline-specific logo or
  shifted placement is introduced.
- [x] A fitting Inline payload with logo mode enabled produces a valid preview
  and both exports instead of the Version 6 versus Version 5 compatibility
  failure.
- [x] Standard no-logo Inline output preserves ordinary ECC-M first fitting and
  may select any fitting version through the new Version 6 ceiling.
- [x] Versions above Version 6 and any unsafe logo geometry remain typed,
  user-visible rejections with exports disabled.
- [x] Deterministic SVG/PNG structure, exact dimensions, native/WASM equality,
  resource baselines, and pinned independent decoding cover Inline Version 6
  with and without the logo.
- [x] Desktop Chromium proves the 100 px real-size Inline preview, updated
  profile guidance, valid logo workflow, and deterministic downloads without
  retries or production payload/logo requests.
- [x] Development policy, testing strategy, generated logo policy, release
  hardening guidance, approved-output matrix policy, and local ticket map record
  the new Inline dimensions and Version 6 ceiling consistently.

## What happened

Inline is currently limited to 90 px SVG / 270 px PNG and Version 5. Logo mode
requires the approved Version 6 minimum, so selecting Inline while retaining the
bundled logo produces a compatibility failure and disables both exports.

## What I expected

Inline should remain a compact profile while being large enough to render the
approved Version 6 logo symbol at the existing six-pixel PNG module scale.

## Steps to reproduce

1. Open the QR generator with the bundled ONE logo enabled.
2. Enter a short payload.
3. Select the Inline output profile.
4. Observe that Inline stops at Version 5 while logo mode requires Version 6.

## Additional context

Version 6 plus the four-module quiet zone has a 49-module logical extent. A
300 px PNG retains an integer six-pixel module scale with symmetric surplus
padding. The paired 100 px SVG keeps the established 3× PNG relationship and is
a round, modest increase over the current 90 px profile. Approval still depends
on the existing deterministic, pinned-decoder, adverse, browser, and release
evidence gates.

## Answer

Inline is now compiled as a 100×100 px SVG / 300×300 px PNG profile with a
Version 6 ceiling. At Version 6 its 49-module logical extent renders at exactly
six PNG pixels per module with three pixels of symmetric background-only
padding on every side. Logo mode therefore uses the same approved ECC-H,
exactly centered 13×4.875-module ONE artwork and 15×7 opaque-white knockout as
the other profiles; no Inline-specific logo geometry was introduced.

The workflow now accepts fitting Inline logo payloads at Version 6, while
ordinary no-logo Inline requests retain ECC-M first fitting through Version 6.
Version 7 and unsafe branding remain typed failures with exports disabled.
Native and WASM artifact hashes, deterministic SVG/PNG structure, resource
budgets, pinned PNG/SVG decoding, and zero-retry Chromium preview/download
coverage include the new Inline contract. The complete approved-output policy
now contains 252 scenarios: 96 required-payload rows and 156 exact-version
rows, partitioned into 151 accepted outputs and 101 expected typed rejections.
The expanded branded geometry experiment decoded all 48/48 Version 6 cases
across Inline, Content, Landing, and Print.
