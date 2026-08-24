# 43 — Place logo upward or fall back to no logo

**What to build:** Replace the currently centered logo behavior with a safe
placement policy that keeps the ONE logo horizontally centered, allows it to
move upward only, and automatically falls back to no-logo output when branding
cannot be placed safely.

**Blocked by:** 40, 41, 42

**Type:** task

**Status:** open

- [ ] Keep the logo horizontally centered in the QR symbol. The vertical search
  may use the exact center as a candidate and then move upward only; it must
  never move the logo downward from center.
- [ ] Structure the QR symbol around the standard regions shown in the provided
  reference: quiet zone, finder patterns, separators, version information when
  present, and data/error-correction codewords. Logo artwork and knockout cells
  must never intersect function modules, separators, quiet zone, fixed dark
  module, timing, alignment, format, or version information.
- [ ] If upward placement reaches too close to the QR edge, quiet zone, finder,
  separator, or another protected region, reject logo placement and switch to
  the same variant without a logo.
- [ ] If the logo cannot be safely placed for the exact payload, selected
  version, ECC, variant, and foreground theme, automatically switch back to
  no-logo output. The fallback must preserve the exact payload and report a
  clear diagnostic explaining why branding was disabled.
- [ ] If no-logo output still cannot fit or cannot satisfy the selected variant,
  show the normal typed capacity/profile error. Do not silently shorten or alter
  the payload.
- [ ] Keep logo size balanced against the QR symbol: large enough to be visibly
  usable, small enough to preserve scan reliability, and backed by exact module
  bounds, obscured-module counts, function-clearance checks, deterministic
  output, and independent decoder evidence.
- [ ] Apply the same geometry and safety policy to magenta and black themes; the
  black QR code uses the black logo, and the magenta QR code uses the magenta
  logo.
- [ ] Update diagnostics to report requested logo state, final logo state,
  fallback reason, selected version, ECC, logo bounds/offset, protected-module
  clearance, obscured data/remainder counts, and final artifact dimensions.
- [ ] Refresh the generated logo placement policy, approved-output matrix,
  adverse evidence, deterministic hashes, and browser/download tests for all
  affected variants and themes.

## Product intent

Branding is desirable but never more important than a correct, scannable QR
code. The app should prefer a safe branded symbol, gracefully remove the logo
when branding would damage correctness, and only show an error when the exact
payload cannot be represented even without the logo.

## Implementation constraints

Logo fallback is a render/workflow policy, not a payload policy. It must never
rewrite user input, change the encoded matrix after rendering starts, or use ECC
percentages as an occlusion budget. Approval requires evidence on the final
production SVG and PNG artifacts, not on a hand-drawn or oracle-only layout.

## Comments
