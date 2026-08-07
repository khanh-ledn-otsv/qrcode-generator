# 21 — Validate release readiness

**What to build:** Produce complete repository-owned automated evidence that the release candidate is safe, private, and usable.

**Blocked by:** 20 — Harden all approved output combinations.

**Status:** resolved

- [x] A local production build and network inspection show no runtime payload or logo requests.
- [x] Clean builds with pinned tools record matching application and artifact hashes.
- [x] Desktop Chromium passes critical paths, downloads, and privacy inspection without retry-hidden correctness failures.
- [x] User guidance explains SVG-first export, physical-size guidance, transparent/logo cautions, and environment-specific validation.
- [x] The release runbook links every repository-owned acceptance criterion to automated results.

## Comments

- 2026-08-07: Owner removed bundle-size, automated accessibility, and manual-test
  evidence from the repository release gate. The readiness report now validates
  reproducible hashes, all configured browser projects, privacy, downloads,
  approved artifacts, and user guidance without collecting manual evidence.
- 2026-08-07: The owner narrowed the configured browser gate to desktop
  Chromium and removed responsive-layout assertions.

## Answer

Ticket 21 is complete under the revised evidence policy. `release:readiness`
builds twice, checks exact hashes, runs desktop Chromium with zero retries,
collects decoder evidence, and validates the automated report. Manual product
testing is intentionally outside the repository and requires no evidence file.
