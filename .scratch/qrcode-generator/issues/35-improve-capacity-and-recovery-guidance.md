# 35 — Improve capacity and recovery guidance

**What to build:** Make capacity diagnostics and recovery suggestions reflect what the selected QR workflow can actually do.

**Blocked by:** none.

**Type:** task

**Priority:** deferred improvement

- [ ] Distinguish profile-ceiling overflow from payloads that cannot fit QR
  Version 40 at all.
- [ ] Recommend Adaptive only when it can encode the payload under the current
  logo policy; otherwise recommend shortening the payload or disabling the
  logo as applicable.
- [ ] Add a mode-aware same-version calculation for additional characters in
  the current whole-payload mode.
- [ ] Display remaining capacity as an estimate when subsequent edits could
  change Numeric, Alphanumeric, Byte, or ECI selection.
- [ ] Cover exact-fit and one-over behavior for all modes, UTF-8+ECI, fixed
  profile ceilings, Adaptive Version 40, and the Adaptive logo Version 11/12
  boundary.

## Context

The current UI accurately reports used/available data bits, but omits the
remaining-character diagnostic required by the development plan. Some fixed
profile failures also suggest Adaptive even when Version-40 capacity or the
approved logo-version boundary guarantees another failure.

