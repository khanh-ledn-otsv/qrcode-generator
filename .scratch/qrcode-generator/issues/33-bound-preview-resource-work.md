# 33 — Bound preview resource work

**What to build:** Keep oversized input and preview artifact generation from doing avoidable work on the browser main thread.

**Blocked by:** none.

**Type:** task

**Priority:** deferred improvement

- [ ] Reject payloads above the 4 KiB UTF-8 limit before cloning them into a
  `PreviewRequest`.
- [ ] Keep the rejection typed, payload-preserving, and covered by an exact
  4 KiB / one-byte-over workflow test.
- [ ] Measure dense Adaptive preview generation in desktop Chromium and record
  the result before choosing an implementation.
- [ ] Avoid eagerly rendering PNG when the interactive preview consumes only
  SVG; generate or cache PNG at download time, or move artifact work off the
  main thread if measurement justifies it.
- [ ] Verify that rapid typing, stale revision rejection, deterministic
  downloads, and object-URL cleanup still pass.

## Context

The current workflow clones the full payload before `qr-core` applies its
defensive limit, and the debounced callback synchronously generates both SVG
and PNG. Neither is currently a correctness failure, but both are worthwhile
browser-resource hardening opportunities.

