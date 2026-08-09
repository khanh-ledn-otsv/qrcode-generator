# 29 — Support adaptive large branded QR output

**What to build:** Add a separate selectable **Adaptive Branded** output variant
that uses deterministic, decode-approved adaptive logo sizing and function-safe
placement on the smallest ECC-H QR version that fits, while deriving a large
enough export canvas to preserve robust final-pixel module scale. Keep Inline,
Content, Landing, and Print available and unchanged.

**Blocked by:** none.

**Type:** task

**Status:** resolved

- [x] `Adaptive Branded` is a distinct fifth output variant alongside Inline,
  Content, Landing, and Print; it does not replace, rename, silently select, or
  become the recommended/default variant in this ticket.
- [x] The four existing output variants retain their compiled dimensions,
  ceilings, selection behavior, deterministic artifact bytes, and current
  Version 6 centered-logo policy.
- [x] Logo mode preserves the payload byte-for-byte, keeps ECC H, and selects
  the smallest fitting QR version admitted by Adaptive Branded instead of
  forcing or assuming Version 6.
- [x] The adaptive placement policy tries the matrix center only when it is
  function-safe, then deterministically searches nearby data/remainder-only
  regions and scales the logo/opaque-white knockout down within an explicit
  reviewed legibility bound.
- [x] No logo artwork or knockout cell intersects a finder, separator, timing,
  alignment, format, version, or fixed-dark module; if no approved candidate
  exists, a typed user-visible rejection keeps both exports disabled.
- [x] The 110-byte ONE news URL from this ticket selects Version 10 at ECC H
  under Adaptive Branded and produces a valid branded preview and both exports
  without shortening, normalizing, or otherwise rewriting the URL.
- [x] Version 10 branded output uses a version-aware large canvas with at least
  eight final PNG pixels per QR module including the fixed four-module quiet
  zone; the target preset is at least 520 px PNG, with 180 px SVG / 540 px PNG
  used if the established 3× relationship and symmetric padding are retained.
- [x] The Version 10 placement experiment evaluates the existing 15×7 knockout
  near the closest function-safe positions (approximately six modules above or
  below center) plus smaller reviewed candidates; production selects only a
  candidate that passes the complete evidence gate.
- [x] The existing Version 6 centered ONE treatment remains deterministic and
  byte-stable unless new evidence deliberately replaces it; no-logo output
  continues to use ordinary ECC-M first fitting in every variant, including
  Adaptive Branded.
- [x] UI diagnostics report selected version, ECC, module scale, final
  dimensions, logo bounds/offset, obscured data/remainder counts, and safety;
  existing variants that cannot fit a request provide an actionable option to
  select Adaptive Branded rather than a dead-end error.
- [x] Native PNG and independently rasterized SVG evidence cover every enabled
  adaptive branded version, placement candidate, required payload class,
  profile/size, background, and intentional typed rejection with deterministic
  hashes and pinned independent decoding.
- [x] Native/WASM byte equality uses a generated synthetic 110-byte URL under
  the repository fixture policy. Native, WASM, resource baselines, adverse
  transforms, and zero-retry desktop Chromium separately cover the supplied
  long URL, Version 6 regression behavior, adaptive sizing/placement,
  deterministic downloads, accessibility, and the rule that production makes
  no payload or logo network request.
- [x] Development policy, testing strategy, generated logo-placement policy,
  approved-output matrix policy, release hardening/readiness guidance, and the
  local implementation map are revised consistently before resolution.

## What happened

The exact URL
`https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya`
contains 110 ASCII bytes. It requires Version 7 at ECC M and Version 10 at ECC
H. Inline therefore exceeds its Version 6 ceiling without or with the logo;
Content accepts the no-logo Version 7 symbol but cannot fit the ECC-H Version
10 symbol; Landing and Print fit Version 10 but reject branding because the
current policy approves exactly centered logo geometry only on Version 6.

## What I expected

The new Adaptive Branded variant should generate a physically larger QR symbol,
reduce the logo's relative footprint as needed, and place it in the nearest
deterministic function-safe region so long URLs can retain ONE branding without
weakening ECC or rewriting payload bytes. Existing variants should remain
available for their current fixed-size workflows.

## Additional context

Version 10 has a 57×57 module matrix and a 65-module logical extent after the
four-module quiet zone on each side. Eight PNG pixels per module require at
least 520 px. A diagnostic geometry scan found function-safe 15×7 knockout
candidates at `(left 21, top 19)` and `(left 21, top 31)`, whose centers are six
modules above or below the matrix center. These coordinates demonstrate
feasibility only; they are not approved production geometry until the same
deterministic native/SVG/WASM, adverse, Chromium, and pinned-decoder campaign
used for the Version 6 treatment passes.

Do not treat ECC percentages as an occlusion budget, silently lower ECC, force
a larger-than-needed QR version, cover protected alignment geometry, or add a
production URL-shortening/network dependency.

Promotion of Adaptive Branded to the recommended/default output—or removal or
consolidation of the four fixed variants—is explicitly deferred to a later
product decision backed by usage and decoder evidence.

## Answer

Implemented Adaptive Branded as a fifth, non-default 180 px SVG / 540 px PNG
profile with a Version 10 ceiling. Logo mode preserves the exact payload, keeps
ECC H and the Version 6 minimum, and selects the first fitting version through
Version 10. The four fixed variants and their Version 6-only centered branding
remain unchanged.

Adaptive branding checks the center first, then searches function-safe integer
module offsets deterministically while considering 13- through 10-module source
widths. Versions 7–10 select the largest 13×4.875-module ONE source six modules
above center; Version 10 uses source `(22, 20.0625, 13, 4.875)` and knockout
`(21, 19, 15, 7)`. A focused pinned-ZXing experiment records all 12 width/offset
candidates: 112/112 renderable PNG/SVG artifacts decoded, and all four centered
candidates were rejected for protected alignment overlap.

The exact 110-byte ONE URL now produces an ECC-H Version 10 branded preview and
deterministic downloads with eight PNG pixels/module, a 520 px rendered symbol,
and 10 px symmetric padding. Native, WASM, and zero-retry Chromium tests verify
repeated artifact bytes and diagnostics for that URL; the Chromium download
independently decodes to the original URL. A generated synthetic 110-byte URL
pins native/WASM byte equality without placing external content in a golden
fixture. The release matrix now owns 316 scenarios (194 decoded, 122 typed
invalid), and adverse evidence owns 35 outcomes including the adaptive long-URL
caution envelope. Existing-profile failures recommend Adaptive Branded without
silently switching the user’s selection.
