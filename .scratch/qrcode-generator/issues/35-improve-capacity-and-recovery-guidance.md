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
- [x] Keep exact used/available bit diagnostics and avoid presenting a
  character-count remainder that can become misleading after Numeric,
  Alphanumeric, Byte, ECI, or mixed-segment replanning.
- [ ] Cover exact-fit and one-over recovery behavior for pure and mixed modes,
  UTF-8+ECI, fixed profile ceilings, Adaptive Version 40, and the Adaptive logo
  Version 11/12 boundary.

## Context

The current UI accurately reports used/available data bits and whether the
symbol uses a pure or mixed segment plan. The accepted development plan does
not require an exact remaining-character count; it explicitly notes that
remaining capacity is only an estimate because edits can change segmentation.
Ticket 37's mixed-mode planner makes a single character estimate even less
stable, so this ticket must not add one merely to satisfy its original wording.

The actionable gap is recovery guidance: some fixed-profile failures still
suggest Adaptive even when Version 40 capacity or the approved Adaptive logo
Version 11 boundary guarantees another failure. Determine recommendations from
the actual Adaptive workflow outcome while preserving the payload exactly.
